use crate::{hooks::Hooks, interfaces::Interfaces};

mod config;
mod features;
mod helpers;
mod hooks;
mod interfaces;
mod traits;
mod types;

#[used]
#[unsafe(link_section = ".init_array")]
static INIT: extern "C" fn() = {
    extern "C" fn init() {
        env_logger::Builder::new()
            .filter_level(log::LevelFilter::Debug)
            .init();
        if let Err(e) = Interfaces::init() {
            log::error!("tf_rs init failed: {e}");
        }
        if let Err(e) = Hooks::init() {
            log::error!("tf_rs init failed: {e}");
        }
    }
    init
};

#[used]
#[unsafe(link_section = ".fini_array")]
static FINI: extern "C" fn() = {
    extern "C" fn fini() {
        log::info!("tf_rs fini");
        Hooks::restore();
    }
    fini
};
