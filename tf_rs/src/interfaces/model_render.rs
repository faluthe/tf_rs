use std::ffi::c_void;

#[derive(Default, Clone)]
pub struct ModelRender {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl ModelRender {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        ModelRender { this, vtable }
    }

    pub fn ptr(&self) -> *mut c_void {
        self.this
    }
}
