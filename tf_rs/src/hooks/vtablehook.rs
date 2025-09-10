use std::{ffi::c_void, ptr};

use anyhow::{Result, anyhow};

use libc::{_SC_PAGESIZE, PROT_READ, PROT_WRITE, mprotect, sysconf};

use crate::hooks::FnSig;

#[derive(Clone)]
pub struct VTableHook {
    vtable: *mut *mut c_void,
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
            self.vtable = *(interface as *mut *mut *mut c_void);
            self.original = FnSig::from_ptr(*self.vtable.add(index), hook);

            let page_size = sysconf(_SC_PAGESIZE) as usize;
            let table_page = (self.vtable as u64 & !(page_size as u64 - 1)) as *mut c_void;

            if mprotect(table_page, page_size, PROT_READ | PROT_WRITE) != 0 {
                return Err(anyhow!("Failed to change memory protection"));
            }

            self.vtable.add(index).write(hook.as_ptr()?);

            if mprotect(table_page, page_size, PROT_READ) != 0 {
                return Err(anyhow!("Failed to restore memory protection"));
            }
        }

        Ok(())
    }

    pub fn restore(&self) -> Result<()> {
        unsafe {
            (self.vtable).add(22).write(self.original.as_ptr()?);
        }

        Ok(())
    }
}
