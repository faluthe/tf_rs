use crate::{
    cfg_enabled,
    config::CONFIG,
    types::{Player, UserCmd, user_cmd::Buttons},
};

pub fn bunnyhop(localplayer: &Player, cmd: *mut UserCmd) {
    if !cfg_enabled!(bunnyhop) {
        return;
    }

    if localplayer.is_on_ground() {
        unsafe { (*cmd).buttons &= !(Buttons::InJump as i32) };
    }
}
