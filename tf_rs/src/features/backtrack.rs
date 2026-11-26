//! Lag compensation abuse: shoot at where enemies were a few ticks ago.
//! See <https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking#Lag_compensation>
//!
//! # How the server lag compensates
//!
//! What we see of other players is always in the past, and a shot lands later still:
//!
//! - An update takes the incoming half of our latency to reach us.
//! - The client draws players `lerp` behind the newest update. Updates arrive in steps, one every
//!   15ms at 66 a second, and moving a player smoothly between two of them needs the next one to
//!   already be here. Otherwise players would either jump from update to update or have to be
//!   guessed ahead and corrected. Staying `cl_interp_ratio` updates behind keeps the time being
//!   drawn between two updates we have. A ratio of 1 stutters when an update is late or lost, 2
//!   covers one lost update but shows everyone one update further back.
//! - Our shot takes the outgoing half of our latency to reach the server.
//!
//! With 50ms of ping split evenly and 15ms of lerp, what's on screen is 40ms behind where the
//! server has players at that moment, and 65ms behind by the time the shot gets there. The ping
//! net_graph shows is only the round trip, outgoing plus incoming; lerp is its own line and adds on
//! top.
//!
//! To make that hit anyway, every command carries `tick_count`, the tick the client was on when it
//! made the command. Before running a shot the server:
//!
//! 1. Takes `tick_count - lerp` as the tick we were looking at. It knows our lerp from the
//!    interpolation cvars we send it.
//! 2. Works out how far back it expected that to be, round trip latency plus lerp from its own
//!    measurement of our connection, capped at `sv_maxunlag`.
//! 3. Uses its own estimate instead if our tick is more than 200ms off from it. Some slack is
//!    needed because ping jitters and lerp is rounded to whole ticks.
//! 4. Moves every player to where they were on that tick, runs the shot and moves them back.
//!
//! # How we abuse it
//!
//! `tick_count` is just a number we send, so any tick inside that 200ms window is accepted. Each
//! enemy update is recorded with its bones, and when shooting `tick_count` is set to the tick of
//! the record closest to the crosshair plus lerp, so the server rewinds exactly to that record.
//! The server keeps its own history, ours is only to know where enemies were and which ticks are
//! still in the window. With the 65ms above, records from now back to 265ms ago are accepted,
//! minus any newer than the model being drawn.
//!
//! Fake ping delays our acknowledgements so the server measures more latency, which moves the
//! window further back. It shows up in net_graph's ping too.

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
        projectile,
    },
    helpers,
    interfaces::{GlobalVars, Interfaces},
    types::{
        ConVar, Flow, Hull, Matrix3x4, Player, UserCmd, Vec3,
        player::{BONE_USED_BY_ANYTHING, BONE_USED_BY_HITBOX, Bones, MAX_BONES},
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
    /// Only exists while hosting, the server's default is used otherwise
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

/// How far behind the newest update enemies are drawn, which the server subtracts from our
/// `tick_count` to get the tick we were looking at. The cvars are clamped like the server clamps
/// them, otherwise our record's tick would be off by the difference
fn lerp() -> f32 {
    let c = cvars();
    let mut rate = c.updaterate.map_or(66.0, |c| c.float());
    if let (Some(min), Some(max)) = (c.min_updaterate, c.max_updaterate) {
        rate = rate.clamp(min.float(), max.float().max(min.float()));
    }
    let mut ratio = c.interp_ratio.map_or(1.0, |c| c.float());
    if let (Some(min), Some(max)) = (c.min_interp_ratio, c.max_interp_ratio)
        && min.float() != -1.0
    {
        ratio = ratio.clamp(min.float(), max.float().max(min.float()));
    }
    let interp = c.interp.map_or(0.1, |c| c.float()).clamp(0.0, 0.5);
    interp.max(ratio / rate.max(1.0))
}

/// Clock used to check records against the window the server will accept
#[derive(Clone, Copy)]
struct Timing {
    /// How far back the server expects to rewind, both directions of latency plus lerp
    correct: f32,
    /// Tick the server will be on when it runs our command, the outgoing latency from now
    server_tick: i32,
    /// Time enemies are drawn at, interpolation keeps them this far behind the newest update
    render_time: f32,
    /// Oldest age a record may have, from the configured length
    max_age: f32,
    globals: &'static GlobalVars,
}

impl Timing {
    fn now(config: &BacktrackConfig) -> Option<Self> {
        let nci = Interfaces::engine_client().get_net_channel_info()?;
        let outgoing = nci.latency(Flow::Outgoing);
        let incoming = nci.latency(Flow::Incoming);
        let maxunlag = cvars().maxunlag.map_or(1.0, |c| c.float());
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
            globals,
        })
    }

    /// How long before the tick the server runs our command on `sim_time` was
    fn age(&self, sim_time: f32) -> f32 {
        let g = self.globals;
        g.ticks_to_time(self.server_tick - g.time_to_ticks(sim_time))
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

/// Where a record's bones put the player
struct Pose {
    bones: Box<Bones>,
    head: Vec3,
    body: Vec3,
    /// Every bone was set up, not only the ones hitboxes use, so the model can be drawn with them
    drawable: bool,
}

pub struct Record {
    pub sim_time: f32,
    /// Collision box, what melee swings trace against
    hull: Hull,
    pose: Pose,
}

/// Records of one player, newest first
type History = VecDeque<Record>;

/// Records of each player by entity index
static RECORDS: Mutex<Vec<History>> = Mutex::new(Vec::new());

/// Bone buffers of dropped records, reused for new ones instead of allocating every tick
static SPARE_BONES: Mutex<Vec<Box<Bones>>> = Mutex::new(Vec::new());

/// Entity index and simulation time of the record we would shoot at
static TARGET: Mutex<Option<(i32, f32)>> = Mutex::new(None);

/// Timing as of when the frame started rendering, none when backtrack is off
static FRAME_TIMING: Mutex<Option<Timing>> = Mutex::new(None);

fn records() -> MutexGuard<'static, Vec<History>> {
    RECORDS.lock().unwrap()
}

/// Stop enemies from being interpolated so they're drawn where the newest update put them,
/// run after the update's entity data is applied. Their movement is choppier, but they're drawn
/// lerp more recently and the drawn model matches the newest record
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

/// Work out the timing records are drawn with this frame once, instead of for every model,
/// run when the frame starts rendering
pub fn start_frame() {
    let config = Config::read().backtrack.clone();
    let timing = config.enabled.then(|| Timing::now(&config)).flatten();
    *FRAME_TIMING.lock().unwrap() = timing;
}

/// Drop a record, keeping its bones to reuse
fn recycle(record: Record, spare: &mut Vec<Box<Bones>>) {
    spare.push(record.pose.bones);
}

/// Drop all of a player's records, keeping their bones to reuse
fn clear(history: &mut History, spare: &mut Vec<Box<Bones>>) {
    history.drain(..).for_each(|r| recycle(r, spare));
}

/// Record every enemy that got a new update, run once the frame's packets are processed
pub fn update_records() {
    let mut records = records();
    let mut spare = SPARE_BONES.lock().unwrap();

    let (config, chams) = {
        let c = Config::read();
        let chams = &c.esp.chams;
        (
            c.backtrack.clone(),
            c.esp.master
                && chams.enabled
                && (chams.backtrack.active() || chams.backtrack_target.active()),
        )
    };
    let localplayer = helpers::get_localplayer().filter(|p| !p.is_dead());
    let (Some(localplayer), true) = (localplayer, config.enabled) else {
        records.iter_mut().for_each(|h| clear(h, &mut spare));
        return;
    };
    let Some(timing) = Timing::now(&config) else {
        return;
    };
    // Shooting only needs the bones hitboxes use, drawing the model needs all of them
    let bone_mask = if chams {
        BONE_USED_BY_ANYTHING
    } else {
        BONE_USED_BY_HITBOX
    };

    let max_clients = Interfaces::engine_client().get_max_clients();
    let len = max_clients as usize + 1;
    records
        .iter_mut()
        .skip(len)
        .for_each(|h| clear(h, &mut spare));
    records.resize_with(len, History::new);

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

        // A record only gets older, drop it once it's past what the server will rewind to
        while player_records
            .back()
            .is_some_and(|r| timing.delta(r.sim_time) <= -WINDOW)
        {
            recycle(player_records.pop_back().unwrap(), &mut spare);
        }

        let sim_time = player.simulation_time();
        if player_records
            .front()
            .is_some_and(|r| r.sim_time == sim_time)
        {
            continue;
        }

        let Some(pose) = pose(&player, sim_time, bone_mask, &mut spare) else {
            continue;
        };
        player_records.push_front(Record {
            sim_time,
            hull: player.hull(),
            pose,
        });
    }
}

/// Set up the bones in `mask` of the update `player` just received
fn pose(player: &Player, sim_time: f32, mask: i32, spare: &mut Vec<Box<Bones>>) -> Option<Pose> {
    // The bones cached this frame are from the interpolated position, not the one just received
    player.invalidate_bone_cache();
    let mut bones = spare
        .pop()
        .unwrap_or_else(|| Box::new([Matrix3x4::default(); MAX_BONES]));
    if !player.setup_bones(&mut bones, mask, sim_time) {
        spare.push(bones);
        return None;
    }
    Some(Pose {
        head: bones[player.head_bone_id()].position(),
        body: bones[1].position(),
        drawable: mask == BONE_USED_BY_ANYTHING,
        bones,
    })
}

/// Which tick a command should use
pub enum Choice {
    /// Pick one from where we're aiming
    Auto,
    /// Leave the tick alone, the aimbot is going for where the target is now
    Keep,
    /// The aimbot picked this entity index and simulation time
    Record(i32, f32),
}

/// Whether lag compensation helps the weapon we're holding. The server only rewinds players for
/// hitscan and melee, projectiles are simulated where everyone really is.
pub fn weapon_is_lag_compensated(localplayer: &Player) -> bool {
    localplayer
        .active_weapon()
        .is_none_or(|w| !projectile::is_projectile_weapon(&w))
}

/// Pick the record closest to where we're aiming, or with a melee weapon one the swing would
/// connect with, and set the tick to it when shooting. A record the aimbot picked is always used
pub fn run(localplayer: &Player, cmd: *mut UserCmd, config: &Config, choice: Choice) {
    let cmd = unsafe { &mut *cmd };
    let target = if !config.backtrack.enabled || !weapon_is_lag_compensated(localplayer) {
        None
    } else {
        match choice {
            Choice::Auto => select(localplayer, cmd, config),
            Choice::Keep => None,
            Choice::Record(index, sim_time) => Some((index, sim_time, true)),
        }
    };
    *TARGET.lock().unwrap() = target.map(|t| (t.0, t.1));

    let Some((_, sim_time, picked)) = target else {
        return;
    };
    // Melee hits land a few ticks after the swing starts, using whichever command is running then
    if picked || (cmd.buttons & Buttons::InAttack as i32) != 0 {
        cmd.tick_count = Interfaces::global_vars().time_to_ticks(sim_time + lerp());
    }
}

/// Entity index and simulation time of the best record, and whether it was picked for melee
fn select(localplayer: &Player, cmd: &UserCmd, config: &Config) -> Option<(i32, f32, bool)> {
    let timing = Timing::now(&config.backtrack)?;
    let records = records();
    let angles = cmd.view_angles;

    if let Some(weapon) = localplayer.active_weapon().filter(|w| w.is_melee()) {
        let eye = melee::eye_at(localplayer, cmd, 0.0, config);
        let swing = Swing::new(&weapon, eye, &angles);
        let knife = weapon.is_knife();
        let entity_list = Interfaces::entity_list();
        // Any record the swing connects with works, one a knife backstabs first, then the
        // newest which relies least on timing
        return records
            .iter()
            .enumerate()
            .flat_map(|(i, player_records)| {
                let i = i as i32;
                // Looked up once per player, not for each of their records
                let yaw = (knife && !player_records.is_empty())
                    .then(|| entity_list.get_client_entity::<Player>(i))
                    .flatten()
                    .map(|p| p.eye_yaw());
                player_records
                    .iter()
                    .filter(move |r| timing.usable(r.sim_time) && swing.hits(&r.hull))
                    .map(move |r| {
                        let backstab = yaw.is_some_and(|yaw| {
                            melee::can_backstab(&eye, &angles, &r.hull.origin, yaw)
                        });
                        (backstab, i, r.sim_time)
                    })
            })
            .max_by(|a, b| a.0.cmp(&b.0).then(a.2.total_cmp(&b.2)))
            .map(|(_, i, sim_time)| (i, sim_time, true));
    }

    // Without a shot to time, the pick only matters to show the target's chams
    let chams = &config.esp.chams;
    let attacking = (cmd.buttons & Buttons::InAttack as i32) != 0;
    let target_chams = config.esp.master && chams.enabled && chams.backtrack_target.active();
    if !(attacking || target_chams) {
        return None;
    }

    let shoot_pos = localplayer.eye_pos();
    records
        .iter()
        .enumerate()
        .flat_map(|(i, player_records)| player_records.iter().map(move |r| (i as i32, r)))
        .filter(|(_, r)| timing.usable(r.sim_time))
        .flat_map(|(i, r)| {
            [r.pose.head, r.pose.body].map(|pos| {
                let angle = helpers::calculate_angle(&shoot_pos, &pos);
                (angles.fov_to(&angle), i, r.sim_time)
            })
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, i, sim_time)| (i, sim_time, false))
}

/// Every player's records, locked once for the melee aimbot to look through all of them
pub struct Lookup {
    records: MutexGuard<'static, Vec<History>>,
    /// None when backtrack is off, no record is usable then
    timing: Option<Timing>,
}

impl Lookup {
    pub fn new(config: &Config) -> Self {
        let timing = config
            .backtrack
            .enabled
            .then(|| Timing::now(&config.backtrack))
            .flatten();
        Lookup {
            records: records(),
            timing,
        }
    }

    /// Collision boxes and simulation times of player `index`'s records that will still be
    /// usable for a command run `lead` seconds from now
    pub fn candidates(&self, index: i32, lead: f32) -> impl Iterator<Item = (Hull, f32)> + '_ {
        let timing = self.timing;
        self.records
            .get(index as usize)
            .into_iter()
            .flatten()
            .filter(move |r| timing.is_some_and(|t| t.usable_in(r.sim_time, lead)))
            .map(|r| (r.hull, r.sim_time))
    }
}

/// Simulation time, head and body of player `index`'s records that can be shot at now, none when
/// backtrack is off
pub fn aim_points(index: i32, config: &Config) -> Vec<(f32, Vec3, Vec3)> {
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
        .filter(|r| timing.usable(r.sim_time))
        .map(|r| (r.sim_time, r.pose.head, r.pose.body))
        .collect()
}

/// Records closer together than this look the same, only one of them is drawn
const MIN_GAP: f32 = 1.0;

fn near(a: &Vec3, b: &Vec3) -> bool {
    let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
    dx * dx + dy * dy + dz * dz < MIN_GAP * MIN_GAP
}

/// One player's records to draw this frame, locked while they're drawn
pub struct Drawn {
    records: MutexGuard<'static, Vec<History>>,
    index: usize,
    timing: Timing,
    /// Simulation time of the record we would shoot at, if it's this player's
    target: Option<f32>,
    /// Where the player's model is drawn, records on top of it aren't
    model_origin: Vec3,
}

/// Records of player `index` that can be shot at, to draw over their model drawn at
/// `model_origin`
pub fn drawn(index: i32, model_origin: Vec3) -> Option<Drawn> {
    let timing = (*FRAME_TIMING.lock().unwrap())?;
    let target = TARGET.lock().unwrap().filter(|t| t.0 == index).map(|t| t.1);
    // Drawing never happens while records are being updated, don't wait on a lock if it somehow does
    let records = RECORDS.try_lock().ok()?;
    records.get(index as usize)?;
    Some(Drawn {
        records,
        index: index as usize,
        timing,
        target,
        model_origin,
    })
}

impl Drawn {
    /// Usable records with bones to draw, oldest first
    fn usable(&self) -> impl Iterator<Item = (&Record, &Bones)> + '_ {
        self.records[self.index]
            .iter()
            .rev()
            .filter(|r| self.timing.usable(r.sim_time))
            .filter(|r| r.pose.drawable)
            .map(|r| (r, &*r.pose.bones))
    }

    /// Bones of every record but the target, oldest first, leaving out ones that would be
    /// drawn over the model or the record before them
    pub fn others(&self) -> impl Iterator<Item = &Bones> + '_ {
        let mut last: Option<Vec3> = None;
        self.usable()
            .filter(move |(r, _)| {
                let origin = r.hull.origin;
                if Some(r.sim_time) == self.target
                    || near(&origin, &self.model_origin)
                    || last.is_some_and(|l| near(&origin, &l))
                {
                    return false;
                }
                last = Some(origin);
                true
            })
            .map(|(_, bones)| bones)
    }

    /// Bones of the record we would shoot at
    pub fn target(&self) -> Option<&Bones> {
        let target = self.target?;
        self.usable()
            .find(|(r, _)| r.sim_time == target)
            .map(|(_, bones)| bones)
    }
}
