use std::{
    ffi::{CStr, CString, c_char, c_void},
    mem, ptr,
};

use libc::{RTLD_NOLOAD, RTLD_NOW, dlopen, dlsym};
use log::debug;

use crate::{helpers::module, interfaces::BIN_PATH, vfunc};

/// sizeof(KeyValues) in the current materialsystem.so, the public SDK header is out of date
const KEY_VALUES_SIZE: i32 = 0x48;

type KeyValuesSystemFn = unsafe extern "C" fn() -> *mut c_void;
type LoadFromBufferFn = extern "C" fn(
    this: *mut c_void,
    resource_name: *const c_char,
    buffer: *const c_char,
    file_system: *mut c_void,
    path_id: *const c_char,
) -> bool;

/// The game's KeyValues parser, borrowed from materialsystem.so
#[derive(Default, Clone)]
pub struct KeyValues {
    system: *mut c_void,
    load_from_buffer: Option<LoadFromBufferFn>,
}

impl KeyValues {
    pub fn new() -> anyhow::Result<Self> {
        let system = unsafe {
            let path = CString::new(format!("{BIN_PATH}libvstdlib.so"))?;
            let handle = dlopen(path.as_ptr(), RTLD_NOLOAD | RTLD_NOW);
            if handle.is_null() {
                anyhow::bail!("Failed to open libvstdlib.so: {}", crate::dlerror_string());
            }

            let sym = dlsym(handle, c"KeyValuesSystem".as_ptr());
            if sym.is_null() {
                anyhow::bail!(
                    "Failed to load KeyValuesSystem: {}",
                    crate::dlerror_string()
                );
            }

            mem::transmute::<*mut c_void, KeyValuesSystemFn>(sym)()
        };

        // KeyValues::LoadFromBuffer(const char*, const char*, IBaseFileSystem*, const char*)
        // is the only function that logs this string
        let segments = module::segments("materialsystem.so");
        let log = module::find_string(&segments, c"KeyValues::LoadFromBuffer(%s%s%s): Begin")
            .ok_or_else(|| anyhow::anyhow!("LoadFromBuffer log string not found"))?;
        let lea = module::find_lea_to(&segments, log)
            .ok_or_else(|| anyhow::anyhow!("LoadFromBuffer log string reference not found"))?;
        let load_from_buffer = module::function_start(&segments, lea)
            .ok_or_else(|| anyhow::anyhow!("LoadFromBuffer prologue not found"))?;

        debug!("Found KeyValues::LoadFromBuffer at {load_from_buffer:#x}");

        Ok(KeyValues {
            system,
            load_from_buffer: Some(unsafe {
                mem::transmute::<usize, LoadFromBufferFn>(load_from_buffer)
            }),
        })
    }

    /// Parses VMT style text into a new KeyValues tree, owned by whoever it is passed to
    pub fn parse_buffer(&self, resource_name: &CStr, text: &CStr) -> Option<*mut c_void> {
        let load_from_buffer = self.load_from_buffer?;
        if self.system.is_null() {
            return None;
        }

        let vtable = unsafe { *(self.system as *mut *mut *mut c_void) };
        let alloc = vfunc!(vtable, 1, (*mut c_void, i32) -> *mut c_void);
        let kv = alloc(self.system, KEY_VALUES_SIZE);
        if kv.is_null() {
            return None;
        }

        // Same as the inlined KeyValues::Init, no name and evaluate conditionals on
        unsafe {
            ptr::write_bytes(kv as *mut u8, 0, KEY_VALUES_SIZE as usize);
            *(kv as *mut i32) = -1;
            *(kv as *mut u8).add(0x22) = 1;
        }

        if !load_from_buffer(
            kv,
            resource_name.as_ptr(),
            text.as_ptr(),
            ptr::null_mut(),
            ptr::null(),
        ) {
            return None;
        }

        Some(kv)
    }
}
