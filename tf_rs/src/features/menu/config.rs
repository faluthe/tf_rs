use std::sync::RwLock;

use nuklear::{
    Icon, Nuklear,
    flags::{EditFlags, LayoutFormat, PanelFlags, TextAlignment},
};

use super::ROW;
use crate::config::Config;

static mut SELECTED_CONFIG: usize = 0;
static NEW_CONFIG_NAME: RwLock<[u8; 256]> = RwLock::new([0; 256]);

pub(super) fn config_tab(nk: &Nuklear, config: &mut Config) {
    {
        if nk.section_with_icon("Configurations", true, Icon::Refresh) {
            Config::refresh_configs();
        }
        let configs = Config::list_configs();

        nk.row_dynamic(220.0, 1);
        if nk.group_begin("Configs", PanelFlags::BORDER) {
            for (i, cfg) in configs.iter().enumerate() {
                nk.layout_row_begin(LayoutFormat::DYNAMIC, ROW, 2)
                    .layout_row_push(0.78);

                let mut selected = (unsafe { SELECTED_CONFIG } == i) as i32;
                if nk.selectable_label(cfg.as_str(), TextAlignment::LEFT, &mut selected) {
                    unsafe {
                        SELECTED_CONFIG = i;
                    }
                    if let Err(e) = config.load(cfg) {
                        log::error!("Failed to load config {cfg}: {e:#}");
                    }
                }

                nk.layout_row_push(0.22);

                if selected != 0 && nk.button_label("Save") {
                    if let Err(e) = config.save(cfg) {
                        log::error!("Failed to save config {cfg}: {e:#}");
                    }
                }

                nk.layout_row_end();
            }
            nk.group_end();
        }
    }

    nk.section("New configuration", false);
    let mut ptr = NEW_CONFIG_NAME.write().unwrap();
    nk.layout_row_begin(LayoutFormat::DYNAMIC, ROW, 2)
        .layout_row_push(0.67)
        .edit_string(EditFlags::EDIT_FIELD, ptr.as_mut_ptr() as *mut i8, 256)
        .layout_row_push(0.33);
    let create = nk.button_label("Create");
    nk.layout_row_end();
    if create {
        let end = ptr.iter().position(|&c| c == 0).unwrap_or(256);
        match str::from_utf8(&ptr[..end]) {
            Ok(name) if !name.is_empty() => {
                if let Err(e) = config.save(name) {
                    log::error!("Failed to create config {name}: {e:#}");
                }
                Config::refresh_configs();
            }
            Ok(_) => log::warn!("Config name is empty, not creating"),
            Err(e) => log::error!("Config name is not valid UTF-8: {e}"),
        }
    }
}
