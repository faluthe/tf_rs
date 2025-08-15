use std::{ffi::c_void, ptr, sync::RwLock};

use log::info;
use once_cell::sync::Lazy;

use crate::interfaces::{Factory, TFBIN_PATH};

pub static I: Lazy<RwLock<Interfaces>> = Lazy::new(|| RwLock::new(Interfaces::default()));

pub struct Interfaces {
    pub client: *mut c_void,
    pub client_mode: *mut c_void,
}

unsafe impl Send for Interfaces {}
unsafe impl Sync for Interfaces {}

impl Default for Interfaces {
    fn default() -> Self {
        Interfaces {
            client: ptr::null_mut(),
            client_mode: ptr::null_mut(),
        }
    }
}

impl Interfaces {
    pub fn init() -> anyhow::Result<()> {
        let client_factory = Factory::new(TFBIN_PATH, "client.so")?;

        let mut w = I.write().unwrap();
        w.client = client_factory.get("VClient017")?;

        /*
         * https://github.com/OthmanAba/TeamFortress2/blob/1b81dded673d49adebf4d0958e52236ecc28a956/tf2_src/game/client/cdll_client_int.cpp#L1255
         * CHLClient::HudProcessInput is just a call to g_pClientMode->ProcessInput. Globals are stored as effective addresses.
         * Effective addresses are 4 byte offsets, offset from the instruction pointer (address of next instruction).
         * Manually calculate the effective address of g_pClientMode and dereference it to get the interface.
         */
        unsafe {
            let before_add = *(w.client as *mut *mut *const c_void);
            let hud_process_input = *(before_add.add(10)) as usize;
            let eaddr = ptr::read_unaligned((hud_process_input + 0x3) as *const u32);
            let ip = hud_process_input + 0x7;
            w.client_mode = ptr::read_unaligned((ip + eaddr as usize) as *const *mut c_void);
            info!("Client mode interface at {:p}", w.client_mode);
        }

        if w.client_mode.is_null() {
            return Err(anyhow::anyhow!("Failed to get client mode interface"));
        }

        Ok(())
    }

    pub fn client_mode() -> *mut c_void {
        I.read().unwrap().client_mode
    }
}
