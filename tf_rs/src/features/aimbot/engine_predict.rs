use std::mem;

use log::info;
use once_cell::sync::Lazy;

use crate::{
    helpers,
    types::{Player, UserCmd},
};

static RESET_INSTANCE_COUNTERS: Lazy<fn() -> ()> = Lazy::new(|| {
    let scan_result = helpers::pattern_scan(
        "client.so",
        "E8 ?? ?? ?? ?? 4C 89 A3 20 16 00 00 4C 89 E7 E8 ?? ?? ?? ??",
    )
    .expect("ResetInstanceCounters pattern not found");

    info!("ResetInstanceCounters found at {:#x}", scan_result);

    unsafe { mem::transmute(scan_result) }
});

pub fn start(localplayer: &Player, cmd: &mut UserCmd) {
    if localplayer.is_dead() {
        return;
    }
}
