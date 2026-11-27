use std::ffi::c_void;

use crate::{features::glow, hooks::Hooks, types::ViewSetup};

/// Runs once the view is drawn and post processing is done, where the game draws its glows
pub extern "C" fn hk_do_post_screen_space_effects(
    this: *mut c_void,
    setup: *const ViewSetup,
) -> bool {
    let rc = Hooks::do_post_screen_space_effects()
        .call_do_post_screen_space_effects(this, setup)
        .expect("Invalid DoPostScreenSpaceEffects function signature");

    if let Some(setup) = unsafe { setup.as_ref() } {
        glow::render(setup);
    }
    rc
}
