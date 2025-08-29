use crate::types::{Player, UserCmd, user_cmd::Buttons};

pub fn bunnyhop(localplayer: &Player, cmd: *mut UserCmd) {
    if localplayer.is_on_ground() {
        unsafe { (*cmd).buttons &= !(Buttons::InJump as i32) };
    }
}
