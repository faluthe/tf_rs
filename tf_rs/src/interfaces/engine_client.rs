use std::ffi::c_void;

#[derive(Default, Clone)]
pub struct EngineClient {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl EngineClient {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        EngineClient { this, vtable }
    }

    pub fn get_localplayer_index(&self) -> i32 {
        let func: extern "C" fn(*mut c_void) -> i32 =
            unsafe { std::mem::transmute(*self.vtable.add(12)) };
        func(self.this)
    }
}
