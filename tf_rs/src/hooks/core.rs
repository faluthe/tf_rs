use std::{ffi::c_void, sync::RwLock};

use anyhow::Context;
use once_cell::sync::Lazy;

use crate::{features::fake_ping, hooks::*, interfaces::Interfaces};

const OVERRIDE_VIEW_INDEX: usize = 17;
const CREATE_MOVE_INDEX: usize = 22;
const PAINT_TRAVERSE_INDEX: usize = 42;
const DRAW_MODEL_EXECUTE_INDEX: usize = 19;
const FRAME_STAGE_NOTIFY_INDEX: usize = 35;
const SEND_DATAGRAM_INDEX: usize = 44;

pub static H: Lazy<RwLock<Hooks>> = Lazy::new(|| RwLock::new(Hooks::default()));

#[derive(Default)]
pub struct Hooks {
    pub client: VmtShadow,
    pub client_mode: VmtShadow,
    pub panel: VmtShadow,
    pub model_render: VmtShadow,
    // The net channel is recreated each connection and hooked when one is found
    pub net_chan: VmtShadow,
    // Hooks of earlier net channels, kept alive as a freed channel may still point at them
    pub retired_net_chans: Vec<VmtShadow>,
    pub poll_event: SDLHook,
    pub swap_window: SDLHook,
}

unsafe impl Send for Hooks {}
unsafe impl Sync for Hooks {}

impl Hooks {
    pub fn init() -> anyhow::Result<()> {
        let mut w = H.write().unwrap();
        w.client_mode = VmtShadow::new(Interfaces::client_mode()).context("ClientMode vtable")?;
        w.client_mode
            .hook(CREATE_MOVE_INDEX, FnSig::CreateMove(hk_create_move))
            .context("CreateMove hook")?;
        w.client_mode
            .hook(OVERRIDE_VIEW_INDEX, FnSig::OverrideView(hk_override_view))
            .context("OverrideView hook")?;
        w.client_mode.install().context("ClientMode vtable")?;
        log::debug!("Hooked CreateMove and OverrideView");

        w.panel = VmtShadow::new(Interfaces::panel().ptr()).context("Panel vtable")?;
        w.panel
            .hook(
                PAINT_TRAVERSE_INDEX,
                FnSig::PaintTraverse(hk_paint_traverse),
            )
            .context("PaintTraverse hook")?;
        w.panel.install().context("Panel vtable")?;
        log::debug!("Hooked PaintTraverse");

        w.model_render =
            VmtShadow::new(Interfaces::model_render().ptr()).context("ModelRender vtable")?;
        w.model_render
            .hook(
                DRAW_MODEL_EXECUTE_INDEX,
                FnSig::DrawModelExecute(hk_draw_model_execute),
            )
            .context("DrawModelExecute hook")?;
        w.model_render.install().context("ModelRender vtable")?;
        log::debug!("Hooked DrawModelExecute");

        w.client = VmtShadow::new(Interfaces::client()).context("Client vtable")?;
        w.client
            .hook(
                FRAME_STAGE_NOTIFY_INDEX,
                FnSig::FrameStageNotify(hk_frame_stage_notify),
            )
            .context("FrameStageNotify hook")?;
        w.client.install().context("Client vtable")?;
        log::debug!("Hooked FrameStageNotify");

        w.poll_event
            .hook("SDL_PollEvent\0", FnSig::PollEvent(hk_poll_event))
            .context("SDL_PollEvent hook")?;
        log::debug!("Hooked SDL_PollEvent");

        w.swap_window
            .hook("SDL_GL_SwapWindow\0", FnSig::SwapWindow(hk_swap_window))
            .context("SDL_GL_SwapWindow hook")?;
        log::debug!("Hooked SDL_GL_SwapWindow");

        Ok(())
    }

    pub fn restore() {
        let mut w = H.write().unwrap();

        let net_chan = restore_net_chan(&mut w);

        let results = [
            ("SendDatagram", net_chan),
            ("ClientMode", w.client_mode.restore()),
            ("PaintTraverse", w.panel.restore()),
            ("DrawModelExecute", w.model_render.restore()),
            ("FrameStageNotify", w.client.restore()),
            ("SDL_PollEvent", w.poll_event.restore()),
            ("SDL_GL_SwapWindow", w.swap_window.restore()),
        ];

        // Log instead of panicking, we're running from .fini_array
        for (name, result) in results {
            match result {
                Ok(()) => log::debug!("Restored {name} hook"),
                Err(e) => log::error!("Failed to restore {name} hook: {e:#}"),
            }
        }
    }

    pub fn create_move() -> FnSig {
        H.read().unwrap().client_mode.original(CREATE_MOVE_INDEX)
    }

    pub fn override_view() -> FnSig {
        H.read().unwrap().client_mode.original(OVERRIDE_VIEW_INDEX)
    }

    pub fn paint_traverse() -> FnSig {
        H.read().unwrap().panel.original(PAINT_TRAVERSE_INDEX)
    }

    pub fn draw_model_execute() -> FnSig {
        H.read()
            .unwrap()
            .model_render
            .original(DRAW_MODEL_EXECUTE_INDEX)
    }

    pub fn frame_stage_notify() -> FnSig {
        H.read().unwrap().client.original(FRAME_STAGE_NOTIFY_INDEX)
    }

    pub fn send_datagram() -> FnSig {
        H.read().unwrap().net_chan.original(SEND_DATAGRAM_INDEX)
    }

    /// Hook SendDatagram of the current net channel `net_chan` unless it already is
    pub fn hook_net_chan(net_chan: *mut c_void) -> anyhow::Result<()> {
        let hooked = |h: &Hooks| h.net_chan.object() == net_chan && h.net_chan.is_active();
        if hooked(&H.read().unwrap()) {
            return Ok(());
        }

        let mut w = H.write().unwrap();
        if hooked(&w) {
            return Ok(());
        }
        let old = std::mem::take(&mut w.net_chan);
        if old.is_installed() {
            w.retired_net_chans.push(old);
        }

        let mut shadow = VmtShadow::new(net_chan).context("NetChan vtable")?;
        shadow
            .hook(
                SEND_DATAGRAM_INDEX,
                FnSig::SendDatagram(fake_ping::hk_send_datagram),
            )
            .context("SendDatagram hook")?;
        shadow.install().context("NetChan vtable")?;
        w.net_chan = shadow;
        log::debug!("Hooked SendDatagram of net channel {net_chan:p}");
        Ok(())
    }

    pub fn poll_event() -> SDLHook {
        H.read().unwrap().poll_event.clone()
    }

    pub fn swap_window() -> SDLHook {
        H.read().unwrap().swap_window.clone()
    }
}

/// Restore the net channel hook, only if that channel is still the live one, an old one was freed
fn restore_net_chan(w: &mut Hooks) -> anyhow::Result<()> {
    if !w.net_chan.is_installed() {
        return Ok(());
    }
    let engine = Interfaces::engine_client();
    let current = engine
        .is_in_game()
        .then(|| engine.get_net_channel_info())
        .flatten()
        .map(|nci| nci.ptr());
    if current != Some(w.net_chan.object()) {
        log::debug!(
            "Net channel {:p} is gone, nothing to restore",
            w.net_chan.object()
        );
        return Ok(());
    }
    w.net_chan.restore()
}
