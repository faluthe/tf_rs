use crate::{
    config::Config,
    globals::Globals,
    types::{Player, UserCmd},
};

mod hitscan;
mod projectile;

pub fn run(localplayer: &Player, cmd: *mut UserCmd, config: &Config) {
    if !config.aimbot.master {
        return;
    }

    let mut globals = Globals::write();

    let Some(weapon) = localplayer.active_weapon() else {
        globals.target = None;
        return;
    };

    let cmd = unsafe { &mut *cmd };

    if weapon.is_hitscan() {
        hitscan::run(localplayer, &weapon, cmd, &mut globals, config);
    } else if weapon.is_projectile() {
        projectile::run(localplayer, &weapon, cmd, &mut globals, config);
    } else {
        globals.target = None;
    }
}
