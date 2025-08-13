use std::{ffi::c_void, ptr};

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

            self.vtable.add(index).write(hook);
        }

        Ok(())
    }

    pub fn restore(&self) {
        unsafe {
            (self.vtable).add(22).write(self.original);
        }
    }
}
