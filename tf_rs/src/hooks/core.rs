use std::sync::RwLock;

use once_cell::sync::Lazy;

use crate::{hooks::*, interfaces::Interfaces};

pub static H: Lazy<RwLock<Hooks>> = Lazy::new(|| RwLock::new(Hooks::default()));

#[derive(Default)]
pub struct Hooks {
    pub create_move: VTableHook,
    pub panel: VmtShadow,
    pub poll_event: SDLHook,
    pub swap_window: SDLHook,
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
        w.panel = VmtShadow::new(Interfaces::panel().ptr())?;
        w.panel.hook(42, FnSig::PaintTraverse(hk_paint_traverse))?;
        w.panel.install()?;
        w.poll_event
            .hook("SDL_PollEvent\0", FnSig::PollEvent(hk_poll_event))?;
        w.swap_window
            .hook("SDL_GL_SwapWindow\0", FnSig::SwapWindow(hk_swap_window))?;

        Ok(())
    }

    pub fn restore() {
        let mut w = H.write().unwrap();

        let results = [
            ("CreateMove", w.create_move.restore()),
            ("PaintTraverse", w.panel.restore()),
            ("SDL_PollEvent", w.poll_event.restore()),
            ("SDL_GL_SwapWindow", w.swap_window.restore()),
        ];

        // Log instead of panicking, we're running from .fini_array
        for (name, result) in results {
            if let Err(e) = result {
                log::error!("Failed to restore {name} hook: {e}");
            }
        }
    }

    pub fn create_move() -> VTableHook {
        H.read().unwrap().create_move.clone()
    }

    pub fn paint_traverse() -> FnSig {
        H.read().unwrap().panel.original(42)
    }

    pub fn poll_event() -> SDLHook {
        H.read().unwrap().poll_event.clone()
    }

    pub fn swap_window() -> SDLHook {
        H.read().unwrap().swap_window.clone()
    }
}
