use crate::{
    config::Config,
    types::{Player, UserCmd, user_cmd::Buttons},
};

static mut WAS_JUMPING: bool = false;

pub fn bunnyhop(localplayer: &Player, cmd: *mut UserCmd, config: &Config) {
    if !config.bunnyhop {
        return;
    }

    let is_jumping = unsafe { (*cmd).buttons & (Buttons::InJump as i32) != 0 };
    let is_on_ground = localplayer.is_on_ground();

    if (!is_on_ground) && unsafe { WAS_JUMPING } {
        unsafe { (*cmd).buttons &= !(Buttons::InJump as i32) };
    }

    unsafe {
        WAS_JUMPING = is_jumping;
    }
}
