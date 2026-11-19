use std::{
    ffi::c_void,
    mem,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{hooks::Hooks, interfaces::Interfaces};

mod config;
mod features;
mod globals;
mod helpers;
mod hooks;
mod interfaces;
mod logger;
mod player_db;
mod traits;
mod types;

#[used]
#[unsafe(link_section = ".init_array")]
static INIT: extern "C" fn() = {
    extern "C" fn init() {
        logger::init();
        if let Err(e) = Interfaces::init() {
            log::error!("tf_rs init failed: {e}");
        }
        player_db::load();
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
        restore_hooks();
        logger::shutdown();
    }
    fini
};

static HOOKS_RESTORED: AtomicBool = AtomicBool::new(false);

fn restore_hooks() {
    if HOOKS_RESTORED.swap(true, Ordering::SeqCst) {
        return;
    }
    log::info!("Restoring hooks");
    Hooks::restore();
}

/// Restore all hooks, then dlclose ourselves. The logger is closed by FINI
/// when the dlclose runs.
pub(crate) fn unload() {
    if HOOKS_RESTORED.load(Ordering::SeqCst) {
        return;
    }
    restore_hooks();
    // Nothing may run after a successful dlclose_self, the logger is closing
    if let Err(e) = unsafe { dlclose_self() } {
        log::error!("tf_rs unload failed: {e}");
    }
}

unsafe fn dlclose_self() -> anyhow::Result<()> {
    unsafe {
        let Some(path) = helpers::lib_path() else {
            anyhow::bail!("dladdr failed");
        };

        // NOLOAD returns the existing handle but still takes a reference
        let handle = libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_NOLOAD);
        if handle.is_null() {
            anyhow::bail!("dlopen failed");
        }
        // Drop that reference; the injector's dlopen reference keeps us mapped
        libc::dlclose(handle);

        /* The final dlclose unmaps this code, so it must not return into it.
         * Use dlclose itself as a thread start routine so it returns into libc.
         * The caller still has to get out of our code before the unmap, which
         * is a few instructions against thread creation plus dlclose. */
        let start: extern "C" fn(*mut c_void) -> *mut c_void = mem::transmute(
            libc::dlclose as unsafe extern "C" fn(*mut c_void) -> libc::c_int,
        );

        log::info!("Unloading");

        let mut attr: libc::pthread_attr_t = mem::zeroed();
        libc::pthread_attr_init(&mut attr);
        libc::pthread_attr_setdetachstate(&mut attr, libc::PTHREAD_CREATE_DETACHED);
        let mut tid: libc::pthread_t = 0;
        let rc = libc::pthread_create(&mut tid, &attr, start, handle);
        libc::pthread_attr_destroy(&mut attr);
        if rc != 0 {
            anyhow::bail!("pthread_create failed: {rc}");
        }
    }

    Ok(())
}
