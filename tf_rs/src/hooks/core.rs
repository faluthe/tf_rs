use std::{ffi::c_void, sync::RwLock};

use log::info;
use once_cell::sync::Lazy;

use crate::{
    hooks::{VTableHook, create_move::hk_create_move, fn_sig::FnSig},
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
            FnSig::CreateMove(hk_create_move),
        )?;
        info!(
            "CreateMove hooked with original {:p} and hook {:p}",
            w.create_move.original, hk_create_move as *const c_void
        );

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
