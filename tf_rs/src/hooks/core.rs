use std::{ffi::c_void, sync::RwLock};

use once_cell::sync::Lazy;

use crate::{
    hooks::{VTableHook, create_move::hk_create_move},
    interfaces::Interfaces,
};

pub static H: Lazy<RwLock<Hooks>> = Lazy::new(|| RwLock::new(Hooks::default()));

#[derive(Default)]
pub struct Hooks {
    pub create_move: VTableHook,
}

unsafe impl Send for Hooks {}
unsafe impl Sync for Hooks {}

impl Hooks {
    pub fn init() -> anyhow::Result<()> {
        let mut w = H.write().unwrap();
        w.create_move.hook(
            Interfaces::client_mode(),
            22,
            hk_create_move as *const c_void,
        )?;

        Ok(())
    }

    pub fn restore() {
        let w = H.write().unwrap();

        w.create_move.restore();
    }

    pub fn create_move() -> VTableHook {
        H.read().unwrap().create_move.clone()
    }
}
