use std::{ffi::c_void, ptr};

use libc::{RTLD_LAZY, RTLD_NOLOAD, dlopen, dlsym};

use crate::hooks::FnSig;

#[derive(Clone)]
pub struct SDLHook {
    pub original: FnSig,
}

impl Default for SDLHook {
    fn default() -> Self {
        SDLHook {
            original: FnSig::None,
        }
    }
}

impl SDLHook {
    pub fn hook(&mut self, symbol: &str, hook: FnSig) -> anyhow::Result<()> {
        unsafe {
            let handle = dlopen(
                c"/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0".as_ptr(),
                RTLD_LAZY | RTLD_NOLOAD,
            );

            if handle.is_null() {
                return Err(anyhow::anyhow!("Failed to open SDL2 library"));
            }

            let func = dlsym(handle, symbol.as_ptr() as *const i8);

            if func.is_null() {
                return Err(anyhow::anyhow!("Failed to find symbol {symbol}"));
            }

            /* The symbols resolve to a wrapper function. They are a just a 2 byte
             * `jmp` and then a SIGNED (?) 4 byte offset relative to the instruction
             * pointer. Adding `ip + the offset` is a pointer to the address of the
             * function that is wrapped. We save the orignal and call it in the hook. */
            let offset = ptr::read_unaligned((func as usize + 2) as *const i32);
            let ptr_to_func = (func as usize + 6 + offset as usize) as *mut *mut c_void;
            self.original = FnSig::from_ptr(*ptr_to_func, hook);

            ptr::write(ptr_to_func, hook.as_ptr()?);
        }

        Ok(())
    }
}
