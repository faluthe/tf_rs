use std::{ffi::c_void, ptr, sync::RwLock};

use log::info;
use once_cell::sync::Lazy;

use crate::interfaces::*;

pub static I: Lazy<RwLock<Interfaces>> = Lazy::new(|| RwLock::new(Interfaces::default()));

pub struct Interfaces {
    pub client: *mut c_void,
    pub client_mode: *mut c_void,
    pub engine_client: EngineClient,
    pub entity_list: EntityList,
    pub panel: Panel,
    pub surface: Surface,
    pub debug_overlay: DebugOverlay,
    pub global_vars: *mut GlobalVars,
}

unsafe impl Send for Interfaces {}
unsafe impl Sync for Interfaces {}

impl Default for Interfaces {
    fn default() -> Self {
        Interfaces {
            client: ptr::null_mut(),
            client_mode: ptr::null_mut(),
            engine_client: EngineClient::default(),
            entity_list: EntityList::default(),
            panel: Panel::default(),
            surface: Surface::default(),
            debug_overlay: DebugOverlay::default(),
            global_vars: ptr::null_mut(),
        }
    }
}

impl Interfaces {
    pub fn init() -> anyhow::Result<()> {
        let client_factory = Factory::new(TFBIN_PATH, "client.so")?;
        let engine_factory = Factory::new(BIN_PATH, "engine.so")?;
        let vgui_factory = Factory::new(BIN_PATH, "vgui2.so")?;
        let surface_factory = Factory::new(BIN_PATH, "vguimatsurface.so")?;

        let mut w = I.write().unwrap();
        w.client = client_factory.get("VClient017")?;
        w.engine_client = EngineClient::new(engine_factory.get("VEngineClient014")?);
        w.entity_list = EntityList::new(client_factory.get("VClientEntityList003")?);
        w.panel = Panel::new(vgui_factory.get("VGUI_Panel009")?);
        w.surface = Surface::new(surface_factory.get("VGUI_Surface030")?);
        w.debug_overlay = DebugOverlay::new(engine_factory.get("VDebugOverlay003")?);

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

        unsafe {
            let before_add = *(w.client as *mut *mut *const c_void);
            let hud_update = *(before_add.add(11)) as usize;
            let eaddr = ptr::read_unaligned((hud_update + 0x16) as *const u32);
            let ip = hud_update + 0x1A;
            w.global_vars = ptr::read_unaligned((ip + eaddr as usize) as *const *mut GlobalVars);
            info!("Global vars interface at {:p}", w.global_vars);
        }

        if w.client_mode.is_null() {
            return Err(anyhow::anyhow!("Failed to get client mode interface"));
        }

        Ok(())
    }

    pub fn client_mode() -> *mut c_void {
        I.read().unwrap().client_mode
    }

    pub fn engine_client() -> EngineClient {
        I.read().unwrap().engine_client.clone()
    }

    pub fn entity_list() -> EntityList {
        I.read().unwrap().entity_list.clone()
    }

    pub fn panel() -> Panel {
        I.read().unwrap().panel.clone()
    }

    pub fn surface() -> Surface {
        I.read().unwrap().surface.clone()
    }

    pub fn debug_overlay() -> DebugOverlay {
        I.read().unwrap().debug_overlay.clone()
    }

    pub fn global_vars() -> &'static GlobalVars {
        unsafe { &*I.read().unwrap().global_vars }
    }
}
