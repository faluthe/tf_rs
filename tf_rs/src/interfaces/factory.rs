use std::{
    ffi::{CString, c_char, c_void},
    mem, ptr,
};

use libc::{RTLD_NOLOAD, RTLD_NOW, dlopen, dlsym};
use log::debug;

pub const TFBIN_PATH: &str = "./tf/bin/linux64/";
pub const _BIN_PATH: &str = "./bin/linux64/";

type CreateInterfaceFn =
    unsafe extern "C" fn(name: *const c_char, return_code: *mut i32) -> *mut c_void;

pub struct Factory {
    create_interface: CreateInterfaceFn,
}

impl Factory {
    pub fn new(base_path: &str, lib: &str) -> anyhow::Result<Self> {
        unsafe {
            let path = format!("{base_path}{lib}");
            let c_path = CString::new(path).unwrap();
            let handle = dlopen(c_path.as_ptr(), RTLD_NOLOAD | RTLD_NOW);

            let sym = if !handle.is_null() {
                dlsym(handle, c"CreateInterface".as_ptr() as _)
            } else {
                ptr::null_mut()
            };

            if sym.is_null() {
                return Err(anyhow::anyhow!(
                    "Failed to load CreateInterface from {}",
                    lib
                ));
            }

            debug!("Loaded CreateInterface from {lib} at {sym:p}");

            Ok(Factory {
                create_interface: mem::transmute::<*mut c_void, CreateInterfaceFn>(sym),
            })
        }
    }

    pub fn get(&self, version: &str) -> anyhow::Result<*mut c_void> {
        let c_version = CString::new(version).unwrap();
        let interface = unsafe { (self.create_interface)(c_version.as_ptr(), ptr::null_mut()) };

        if interface.is_null() {
            return Err(anyhow::anyhow!("Failed to get interface for {}", version));
        }

        debug!("Got interface {version} at {interface:p}");

        Ok(interface)
    }
}
