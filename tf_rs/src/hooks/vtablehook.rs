use std::{ffi::c_void, ptr};

use anyhow::{Result, anyhow};

use libc::{_SC_PAGESIZE, PROT_READ, PROT_WRITE, mprotect, sysconf};

use crate::hooks::FnSig;

#[derive(Clone)]
pub struct VTableHook {
    vtable: *mut *mut c_void,
    index: usize,
    pub original: FnSig,
}

impl Default for VTableHook {
    fn default() -> Self {
        VTableHook {
            vtable: ptr::null_mut(),
            index: 0,
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
            self.index = index;
            self.original = FnSig::from_ptr(*self.vtable.add(index), hook);

            self.write_entry(hook.as_ptr()?)?;
        }

        Ok(())
    }

    pub fn restore(&self) -> Result<()> {
        // Never hooked
        if self.vtable.is_null() {
            return Ok(());
        }

        unsafe { self.write_entry(self.original.as_ptr()?) }
    }

    unsafe fn write_entry(&self, func: *mut c_void) -> Result<()> {
        unsafe {
            let entry = self.vtable.add(self.index);

            // Protect the page containing the entry itself, not the start of the vtable
            let page_size = sysconf(_SC_PAGESIZE) as usize;
            let entry_page = (entry as usize & !(page_size - 1)) as *mut c_void;

            if mprotect(entry_page, page_size, PROT_READ | PROT_WRITE) != 0 {
                return Err(anyhow!("Failed to change memory protection"));
            }

            entry.write(func);

            if mprotect(entry_page, page_size, PROT_READ) != 0 {
                return Err(anyhow!("Failed to restore memory protection"));
            }
        }

        Ok(())
    }
}
