//! Lag compensation abuse: shoot at where enemies were a few ticks ago.
//! See https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking#Lag_compensation
//!
//! The server rewinds every player to `cmd.tick_count - lerp` before running a shot, as long as
//! that tick is within 200ms of what it expects from our latency. Each enemy update is recorded
//! with its bones, and when shooting the record closest to the crosshair is picked as the tick.

use std::{
    collections::VecDeque,
    sync::{Mutex, MutexGuard},
};

use once_cell::sync::{Lazy, OnceCell};

use crate::{
    config::Config,
    helpers,
    interfaces::Interfaces,
    types::{
        ConVar, Flow, Matrix3x4, Player, UserCmd, Vec3,
        player::{BONE_USED_BY_ANYTHING, MAX_BONES},
        user_cmd::Buttons,
    },
};

/// How far the server allows the rewound tick to be from the one it expects
const WINDOW: f32 = 0.2;

pub struct Record {
    pub sim_time: f32,
    pub bones: Box<[Matrix3x4; MAX_BONES]>,
    head: Vec3,
    body: Vec3,
}

/// Records of each player by entity index, newest first
static RECORDS: Lazy<Mutex<Vec<VecDeque<Record>>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn records() -> MutexGuard<'static, Vec<VecDeque<Record>>> {
    RECORDS.lock().unwrap()
}

struct Cvars {
    updaterate: Option<ConVar>,
    min_updaterate: Option<ConVar>,
    max_updaterate: Option<ConVar>,
    interp: Option<ConVar>,
    interp_ratio: Option<ConVar>,
    min_interp_ratio: Option<ConVar>,
    max_interp_ratio: Option<ConVar>,
    // Only exists while hosting, the server's default is used otherwise
    maxunlag: Option<ConVar>,
}

fn cvars() -> &'static Cvars {
    static CVARS: OnceCell<Cvars> = OnceCell::new();
    CVARS.get_or_init(|| {
        let cvar = Interfaces::cvar();
        Cvars {
            updaterate: cvar.find_var("cl_updaterate"),
            min_updaterate: cvar.find_var("sv_minupdaterate"),
            max_updaterate: cvar.find_var("sv_maxupdaterate"),
            interp: cvar.find_var("cl_interp"),
            interp_ratio: cvar.find_var("cl_interp_ratio"),
            min_interp_ratio: cvar.find_var("sv_client_min_interp_ratio"),
            max_interp_ratio: cvar.find_var("sv_client_max_interp_ratio"),
            maxunlag: cvar.find_var("sv_maxunlag"),
        }
    })
}

fn value(cvar: Option<ConVar>, default: f32) -> f32 {
    cvar.map_or(default, |c| c.float())
}

/// Interpolation the server lag compensates for, computed like it does from our cvars
fn lerp() -> f32 {
    let c = cvars();
    let mut rate = value(c.updaterate, 66.0);
    if let (Some(min), Some(max)) = (c.min_updaterate, c.max_updaterate) {
        rate = rate.clamp(min.float(), max.float().max(min.float()));
    }
    let mut ratio = value(c.interp_ratio, 1.0);
    if let (Some(min), Some(max)) = (c.min_interp_ratio, c.max_interp_ratio)
        && min.float() != -1.0
    {
        ratio = ratio.clamp(min.float(), max.float().max(min.float()));
    }
    let interp = value(c.interp, 0.1).clamp(0.0, 0.5);
    interp.max(ratio / rate.max(1.0))
}

fn time_to_ticks(time: f32) -> i32 {
    (0.5 + time / Interfaces::global_vars().interval_per_tick) as i32
}

fn ticks_to_time(ticks: i32) -> f32 {
    Interfaces::global_vars().interval_per_tick * ticks as f32
}

/// Clock used to check records against the window the server will accept
struct Timing {
    // How far back the server expects to rewind
    correct: f32,
    // Tick the server will be on when it runs our command
    server_tick: i32,
}

impl Timing {
    fn now() -> Option<Self> {
        let nci = Interfaces::engine_client().get_net_channel_info()?;
        let outgoing = nci.latency(Flow::Outgoing);
        let incoming = nci.latency(Flow::Incoming);
        let maxunlag = value(cvars().maxunlag, 1.0);
        Some(Timing {
            correct: (outgoing + incoming + lerp()).clamp(0.0, maxunlag),
            server_tick: Interfaces::global_vars().tickcount + time_to_ticks(outgoing),
        })
    }

    /// How far the record at `sim_time` is from what the server expects, negative when too old
    fn delta(&self, sim_time: f32) -> f32 {
        self.correct - ticks_to_time(self.server_tick - time_to_ticks(sim_time))
    }

    fn valid(&self, sim_time: f32) -> bool {
        self.delta(sim_time).abs() < WINDOW
    }
}

fn enabled() -> bool {
    Config::read().backtrack.enabled
}

/// Record every enemy that got a new update, run once the frame's packets are processed
pub fn update_records() {
    let mut records = records();

    let localplayer = helpers::get_localplayer().filter(|p| !p.is_dead());
    let (Some(localplayer), true) = (localplayer, enabled()) else {
        records.iter_mut().for_each(VecDeque::clear);
        return;
    };
    let Some(timing) = Timing::now() else {
        return;
    };

    let max_clients = Interfaces::engine_client().get_max_clients();
    records.resize_with(max_clients as usize + 1, VecDeque::new);

    for i in 1..=max_clients {
        let player_records = &mut records[i as usize];

        let player = Interfaces::entity_list()
            .get_client_entity::<Player>(i)
            .filter(|p| {
                *p != localplayer
                    && !p.is_dormant()
                    && !p.is_dead()
                    && p.team() != localplayer.team()
            });
        let Some(player) = player else {
            player_records.clear();
            continue;
        };

        let sim_time = player.simulation_time();
        if player_records
            .front()
            .is_some_and(|r| r.sim_time == sim_time)
        {
            continue;
        }

        // The bones cached this frame are from the interpolated position, not the one just received
        player.invalidate_bone_cache();
        let mut bones = Box::new([Matrix3x4::default(); MAX_BONES]);
        if !player.setup_bones(&mut bones, BONE_USED_BY_ANYTHING, sim_time) {
            continue;
        }

        player_records.push_front(Record {
            sim_time,
            head: bones[player.head_bone_id()].position(),
            body: bones[1].position(),
            bones,
        });

        // A record only gets older, drop it once it's past what the server will rewind to
        while player_records
            .back()
            .is_some_and(|r| timing.delta(r.sim_time) <= -WINDOW)
        {
            player_records.pop_back();
        }
    }
}

/// When shooting, set the tick to the record closest to where we're aiming
pub fn run(localplayer: &Player, cmd: *mut UserCmd) {
    let cmd = unsafe { &mut *cmd };
    if !enabled() || (cmd.buttons & Buttons::InAttack as i32) == 0 {
        return;
    }
    let Some(timing) = Timing::now() else {
        return;
    };

    let shoot_pos = localplayer.eye_pos();
    let records = records();
    let best = records
        .iter()
        .flatten()
        .filter(|r| timing.valid(r.sim_time))
        .flat_map(|r| {
            [r.head, r.body].map(|pos| {
                let angle = helpers::calculate_angle(&shoot_pos, &pos);
                (cmd.view_angles.fov_to(&angle), r.sim_time)
            })
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));

    if let Some((_, sim_time)) = best {
        cmd.tick_count = time_to_ticks(sim_time + lerp());
    }
}

/// Bones of each record of player `index` the server would still rewind to, oldest first
pub fn valid_bones(index: i32, mut f: impl FnMut(&[Matrix3x4; MAX_BONES])) {
    if !enabled() {
        return;
    }
    let Some(timing) = Timing::now() else {
        return;
    };
    // Drawing never happens while records are being updated, don't wait on a lock if it somehow does
    let Ok(records) = RECORDS.try_lock() else {
        return;
    };
    let Some(player_records) = records.get(index as usize) else {
        return;
    };
    for record in player_records.iter().rev() {
        if timing.valid(record.sim_time) {
            f(&record.bones);
        }
    }
}
