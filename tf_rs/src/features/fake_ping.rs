//! Fake latency: send packets acknowledging what the server sent a while ago instead of the
//! newest, so it measures a higher ping and lag compensates further back.

use std::{
    collections::VecDeque,
    ffi::c_void,
    sync::{
        Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

use crate::{config::BacktrackConfig, hooks::Hooks, interfaces::Interfaces, types::NetChannelInfo};

/// The server clamps lag compensation to sv_maxunlag (1s), leave room for the real latency
pub const MAX_MS: i32 = 800;
const MAX_SEQUENCES: usize = 2048;

/// Milliseconds currently added, read from SendDatagram where taking the config lock isn't safe
static FAKE_MS: AtomicU32 = AtomicU32::new(0);

struct Sequence {
    in_sequence: i32,
    in_reliable_state: i32,
    realtime: f32,
}

struct State {
    // Newest first
    sequences: VecDeque<Sequence>,
    net_chan: usize,
}

static STATE: Mutex<State> = Mutex::new(State {
    sequences: VecDeque::new(),
    net_chan: 0,
});

/// Fake latency being added, in seconds
pub fn latency() -> f32 {
    FAKE_MS.load(Ordering::Relaxed) as f32 / 1000.0
}

/// Keep the net channel hooked and remember each sequence received, run every CreateMove
pub fn update(config: &BacktrackConfig) {
    let ms = if config.enabled {
        config.fake_ping.clamp(0, MAX_MS)
    } else {
        0
    };
    FAKE_MS.store(ms as u32, Ordering::Relaxed);

    let Some(nci) = Interfaces::engine_client().get_net_channel_info() else {
        return;
    };
    if let Err(e) = Hooks::hook_net_chan(nci.ptr()) {
        log::error!("Failed to hook net channel: {e:#}");
        return;
    }

    let mut state = STATE.lock().unwrap();
    // A new connection starts its sequences over
    if state.net_chan != nci.ptr() as usize {
        state.net_chan = nci.ptr() as usize;
        state.sequences.clear();
    }

    let in_sequence = nci.in_sequence();
    if state
        .sequences
        .front()
        .is_none_or(|s| in_sequence > s.in_sequence)
    {
        state.sequences.push_front(Sequence {
            in_sequence,
            in_reliable_state: nci.in_reliable_state(),
            realtime: Interfaces::global_vars().realtime,
        });
        state.sequences.truncate(MAX_SEQUENCES);
    }
}

/// Swap in the newest sequence that is at least the fake latency old, returns what to restore
fn apply(nci: &NetChannelInfo) -> Option<(i32, i32)> {
    let fake = latency();
    if fake <= 0.0 {
        return None;
    }
    // Never wait on the lock from inside the engine's send
    let state = STATE.try_lock().ok()?;
    if state.net_chan != nci.ptr() as usize {
        return None;
    }
    let now = Interfaces::global_vars().realtime;
    let old = state.sequences.iter().find(|s| now - s.realtime >= fake)?;

    let saved = (nci.in_sequence(), nci.in_reliable_state());
    nci.set_in_sequence(old.in_sequence);
    nci.set_in_reliable_state(old.in_reliable_state);
    Some(saved)
}

pub extern "C" fn hk_send_datagram(this: *mut c_void, datagram: *mut c_void) -> i32 {
    let call_original = || {
        Hooks::send_datagram()
            .call_send_datagram(this, datagram)
            .expect("Invalid SendDatagram function signature")
    };

    // A datagram passed in is a one off send (e.g. during connection), not the regular packet
    if !datagram.is_null() {
        return call_original();
    }
    let Some(nci) = NetChannelInfo::new(this) else {
        return call_original();
    };
    let Some((in_sequence, in_reliable_state)) = apply(&nci) else {
        return call_original();
    };

    let rc = call_original();
    nci.set_in_sequence(in_sequence);
    nci.set_in_reliable_state(in_reliable_state);
    rc
}
