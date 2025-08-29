use std::{ffi::c_void, ptr};

use anyhow::anyhow;

use libc::{_SC_PAGESIZE, PROT_READ, PROT_WRITE, mprotect, sysconf};

use crate::hooks::fn_sig::FnSig;

#[derive(Clone)]
pub struct VTableHook {
    // Note that T is expected to be a function pointer, so vtable is a list of function pointers
    vtable: *mut FnSig,
    pub original: FnSig,
}

impl Default for VTableHook {
    fn default() -> Self {
        VTableHook {
            vtable: ptr::null_mut(),
            original: FnSig::None,
        }
    }
}

impl VTableHook {
    pub fn hook(
        &mut self,
        interface: *mut c_void,
        index: usize,
        hook: FnSig,
    ) -> anyhow::Result<()> {
        unsafe {
            self.vtable = *(interface as *mut *mut FnSig);
            self.original = *self.vtable.add(index);

            let page_size = sysconf(_SC_PAGESIZE) as usize;
            let table_page = (self.vtable as u64 & !(page_size as u64 - 1)) as *mut c_void;

            if mprotect(table_page, page_size, PROT_READ | PROT_WRITE) != 0 {
                return Err(anyhow!("Failed to change memory protection"));
            }

            self.vtable.add(index).write(hook);

            if mprotect(table_page, page_size, PROT_READ) != 0 {
                return Err(anyhow!("Failed to restore memory protection"));
            }
        }

        Ok(())
    }

    pub fn restore(&self) {
        unsafe {
            (self.vtable).add(22).write(self.original);
        }
    }
}
