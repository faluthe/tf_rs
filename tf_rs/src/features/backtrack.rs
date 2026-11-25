//! Lag compensation abuse: shoot at where enemies were a few ticks ago.
//! See https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking#Lag_compensation
//!
//! The server rewinds every player to `cmd.tick_count - lerp` before running a shot, as long as
//! that tick is within 200ms of what it expects from our latency. Each enemy update is recorded
//! with its bones, and when shooting the record closest to the crosshair is picked as the tick.
//! Fake ping delays our acknowledgements so the server thinks we lag more and rewinds further.

use std::{
    collections::VecDeque,
    sync::{Mutex, MutexGuard},
};

use once_cell::sync::OnceCell;

use crate::{
    config::{BacktrackConfig, Config},
    features::{
        fake_ping,
        melee::{self, Swing},
    },
    helpers,
    interfaces::Interfaces,
    types::{
        ConVar, Flow, Hull, Matrix3x4, Player, UserCmd, Vec3,
        player::{BONE_USED_BY_ANYTHING, Bones, MAX_BONES},
        user_cmd::Buttons,
    },
};

/// How far the server allows the rewound tick to be from the one it expects
const WINDOW: f32 = 0.2;

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

/// Clock used to check records against the window the server will accept
struct Timing {
    // How far back the server expects to rewind
    correct: f32,
    // Tick the server will be on when it runs our command
    server_tick: i32,
    // Time enemies are drawn at, interpolation keeps them this far behind the newest update
    render_time: f32,
    // Oldest age a record may have, from the configured length
    max_age: f32,
}

impl Timing {
    fn now(config: &BacktrackConfig) -> Option<Self> {
        let nci = Interfaces::engine_client().get_net_channel_info()?;
        let outgoing = nci.latency(Flow::Outgoing);
        let incoming = nci.latency(Flow::Incoming);
        let maxunlag = value(cvars().maxunlag, 1.0);
        let lerp = lerp();
        let globals = Interfaces::global_vars();
        let correct = (outgoing + incoming + fake_ping::latency() + lerp).clamp(0.0, maxunlag);
        Some(Timing {
            correct,
            server_tick: globals.tickcount + globals.time_to_ticks(outgoing),
            render_time: if config.disable_interpolation {
                f32::MAX
            } else {
                globals.curtime - lerp
            },
            max_age: (correct - WINDOW).max(0.0) + config.length.max(0) as f32 / 1000.0,
        })
    }

    /// How long before the tick the server runs our command on `sim_time` was
    fn age(&self, sim_time: f32) -> f32 {
        let globals = Interfaces::global_vars();
        globals.ticks_to_time(self.server_tick - globals.time_to_ticks(sim_time))
    }

    /// How far the record at `sim_time` is from what the server expects, negative when too old
    fn delta(&self, sim_time: f32) -> f32 {
        self.correct - self.age(sim_time)
    }

    /// The server will rewind to the record, it's within the configured length and it isn't
    /// ahead of the drawn model
    fn usable(&self, sim_time: f32) -> bool {
        self.usable_in(sim_time, 0.0)
    }

    /// Whether the record will still be usable for a command run `lead` seconds from now
    fn usable_in(&self, sim_time: f32, lead: f32) -> bool {
        let age = self.age(sim_time) + lead;
        (self.correct - age).abs() < WINDOW && age <= self.max_age && sim_time <= self.render_time
    }
}

pub struct Record {
    pub sim_time: f32,
    pub bones: Box<Bones>,
    head: Vec3,
    body: Vec3,
    // Collision box, what melee swings trace against
    hull: Hull,
}

/// Records of one player, newest first
type History = VecDeque<Record>;

/// Records of each player by entity index
static RECORDS: Mutex<Vec<History>> = Mutex::new(Vec::new());

/// Bone buffers of dropped records, reused for new ones instead of allocating every tick
static SPARE_BONES: Mutex<Vec<Box<Bones>>> = Mutex::new(Vec::new());

/// Entity index and simulation time of the record we would shoot at
static TARGET: Mutex<Option<(i32, f32)>> = Mutex::new(None);

fn records() -> MutexGuard<'static, Vec<History>> {
    RECORDS.lock().unwrap()
}

/// Stop enemies from being interpolated so they're drawn where the newest update put them,
/// run after the update's entity data is applied
pub fn disable_interpolation() {
    let config = Config::read().backtrack.clone();
    if !config.enabled || !config.disable_interpolation {
        return;
    }
    let Some(localplayer) = helpers::get_localplayer() else {
        return;
    };

    for i in 1..=Interfaces::engine_client().get_max_clients() {
        let Some(player) = Interfaces::entity_list().get_client_entity::<Player>(i) else {
            continue;
        };
        if player == localplayer || player.is_dormant() || player.team() == localplayer.team() {
            continue;
        }
        player.disable_interpolation();
    }
}

/// Drop all of a player's records, keeping their bones to reuse
fn clear(history: &mut History, spare: &mut Vec<Box<Bones>>) {
    spare.extend(history.drain(..).map(|r| r.bones));
}

/// Record every enemy that got a new update, run once the frame's packets are processed
pub fn update_records() {
    let mut records = records();
    let mut spare = SPARE_BONES.lock().unwrap();

    let (config, melee_aimbot) = {
        let c = Config::read();
        (c.backtrack.clone(), c.aimbot.master && c.aimbot.melee)
    };
    let localplayer = helpers::get_localplayer().filter(|p| !p.is_dead());
    // The melee aimbot uses records to tell how enemies are moving
    let (Some(localplayer), true) = (localplayer, config.enabled || melee_aimbot) else {
        records.iter_mut().for_each(|h| clear(h, &mut spare));
        return;
    };
    let Some(timing) = Timing::now(&config) else {
        return;
    };

    let max_clients = Interfaces::engine_client().get_max_clients();
    records.resize_with(max_clients as usize + 1, History::new);

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
            clear(player_records, &mut spare);
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
        let mut bones = spare
            .pop()
            .unwrap_or_else(|| Box::new([Matrix3x4::default(); MAX_BONES]));
        if !player.setup_bones(&mut bones, BONE_USED_BY_ANYTHING, sim_time) {
            spare.push(bones);
            continue;
        }

        player_records.push_front(Record {
            sim_time,
            head: bones[player.head_bone_id()].position(),
            body: bones[1].position(),
            hull: player.hull(),
            bones,
        });

        // A record only gets older, drop it once it's past what the server will rewind to
        while player_records
            .back()
            .is_some_and(|r| timing.delta(r.sim_time) <= -WINDOW)
        {
            spare.extend(player_records.pop_back().map(|r| r.bones));
        }
    }
}

/// Which tick a command should use
pub enum Choice {
    /// Pick one from where we're aiming
    Auto,
    /// Leave the tick alone, the melee aimbot is going for where the player is now
    Keep,
    /// The melee aimbot picked this entity index and simulation time
    Record(i32, f32),
}

/// Pick the record closest to where we're aiming, or with a melee weapon one the swing would
/// connect with, and set the tick to it when shooting
pub fn run(localplayer: &Player, cmd: *mut UserCmd, config: &Config, choice: Choice) {
    let cmd = unsafe { &mut *cmd };
    let target = if !config.backtrack.enabled {
        None
    } else {
        match choice {
            Choice::Auto => select(localplayer, cmd, &config.backtrack),
            Choice::Keep => None,
            Choice::Record(index, sim_time) => Some((index, sim_time, true)),
        }
    };
    *TARGET.lock().unwrap() = target.map(|t| (t.0, t.1));

    let Some((_, sim_time, melee)) = target else {
        return;
    };
    // Melee hits land a few ticks after the swing starts, using whichever command is running then
    if melee || (cmd.buttons & Buttons::InAttack as i32) != 0 {
        cmd.tick_count = Interfaces::global_vars().time_to_ticks(sim_time + lerp());
    }
}

/// Entity index and simulation time of the best record, and whether it was picked for melee
fn select(
    localplayer: &Player,
    cmd: &UserCmd,
    config: &BacktrackConfig,
) -> Option<(i32, f32, bool)> {
    let timing = Timing::now(config)?;
    let records = records();
    let usable = records
        .iter()
        .enumerate()
        .flat_map(|(i, player_records)| player_records.iter().map(move |r| (i as i32, r)))
        .filter(|(_, r)| timing.usable(r.sim_time));

    if let Some(weapon) = localplayer.active_weapon().filter(|w| w.is_melee()) {
        let eye = melee::eye_at(localplayer, 0.0);
        let swing = Swing::new(localplayer, &weapon, &cmd.view_angles, 0.0)?;
        let knife = weapon.is_knife();
        let backstabs = |i: i32, r: &Record| {
            knife
                && Interfaces::entity_list()
                    .get_client_entity::<Player>(i)
                    .is_some_and(|p| {
                        melee::can_backstab(&eye, &cmd.view_angles, &r.hull.origin, p.eye_yaw())
                    })
        };
        // Any record the swing connects with works, one a knife backstabs first, then the
        // newest which relies least on timing
        return usable
            .filter(|(_, r)| swing.hits(&r.hull))
            .map(|(i, r)| (backstabs(i, r), i, r.sim_time))
            .max_by(|a, b| a.0.cmp(&b.0).then(a.2.total_cmp(&b.2)))
            .map(|(_, i, sim_time)| (i, sim_time, true));
    }

    let shoot_pos = localplayer.eye_pos();
    usable
        .flat_map(|(i, r)| {
            [r.head, r.body].map(|pos| {
                let angle = helpers::calculate_angle(&shoot_pos, &pos);
                (cmd.view_angles.fov_to(&angle), i, r.sim_time)
            })
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, i, sim_time)| (i, sim_time, false))
}

/// Collision boxes and simulation times of player `index`'s records that will still be usable
/// for a command run `lead` seconds from now, none when backtrack is off
pub fn candidates(index: i32, lead: f32, config: &Config) -> Vec<(Hull, f32)> {
    if !config.backtrack.enabled {
        return Vec::new();
    }
    let Some(timing) = Timing::now(&config.backtrack) else {
        return Vec::new();
    };
    let records = records();
    records
        .get(index as usize)
        .into_iter()
        .flatten()
        .filter(|r| timing.usable_in(r.sim_time, lead))
        .map(|r| (r.hull, r.sim_time))
        .collect()
}

/// How player `index` has been moving between their two newest updates
pub fn velocity(index: i32) -> Option<Vec3> {
    let records = records();
    let mut player_records = records.get(index as usize)?.iter();
    let (new, old) = (player_records.next()?, player_records.next()?);
    let dt = new.sim_time - old.sim_time;
    // Too far apart and it's not one movement anymore
    if dt <= 0.0 || dt > 0.25 {
        return None;
    }
    let (a, b) = (new.hull.origin, old.hull.origin);
    Some(Vec3::new(
        (a.x - b.x) / dt,
        (a.y - b.y) / dt,
        (a.z - b.z) / dt,
    ))
}

/// Bones of each record of player `index` that can be shot at, oldest first with the record
/// we would shoot at last, and whether it's that record
pub fn valid_bones(index: i32, config: &BacktrackConfig, mut f: impl FnMut(&Bones, bool)) {
    let Some(timing) = Timing::now(config) else {
        return;
    };
    let target = *TARGET.lock().unwrap();
    // Drawing never happens while records are being updated, don't wait on a lock if it somehow does
    let Ok(records) = RECORDS.try_lock() else {
        return;
    };
    let Some(player_records) = records.get(index as usize) else {
        return;
    };
    let mut target_record = None;
    for record in player_records.iter().rev() {
        if !timing.usable(record.sim_time) {
            continue;
        }
        if target == Some((index, record.sim_time)) {
            target_record = Some(record);
        } else {
            f(&record.bones, false);
        }
    }
    // Last so it's drawn over the others
    if let Some(record) = target_record {
        f(&record.bones, true);
    }
}
