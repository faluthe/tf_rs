use std::{ffi::c_void, ptr};

use anyhow::anyhow;

use libc::{_SC_PAGESIZE, PROT_READ, PROT_WRITE, mprotect, sysconf};

#[derive(Clone)]
pub struct VTableHook {
    vtable: *mut *const c_void,
    pub original: *const c_void,
}

impl Default for VTableHook {
    fn default() -> Self {
        VTableHook {
            vtable: ptr::null_mut(),
            original: ptr::null(),
        }
    }
}

impl VTableHook {
    pub fn hook(
        &mut self,
        interface: *mut c_void,
        index: usize,
        hook: *const c_void,
    ) -> anyhow::Result<()> {
        unsafe {
            self.vtable = *(interface as *mut *mut *const c_void);
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
