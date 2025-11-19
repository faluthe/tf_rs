use crate::{
    config::Config,
    types::{Player, UserCmd, user_cmd::Buttons},
};

pub fn bunnyhop(localplayer: &Player, cmd: *mut UserCmd, config: &Config) {
    if config.bunnyhop == 0 {
        return;
    }

    if localplayer.is_on_ground() {
        unsafe { (*cmd).buttons &= !(Buttons::InJump as i32) };
    }
}
