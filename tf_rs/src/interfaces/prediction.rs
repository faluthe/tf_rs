use std::ffi::c_void;

/// CPrediction, only its state flags
#[derive(Default, Clone)]
pub struct Prediction {
    this: *mut c_void,
}

impl Prediction {
    pub fn new(this: *mut c_void) -> Self {
        Prediction { this }
    }

    fn flag(&self, offset: usize) -> *mut bool {
        unsafe { (self.this as *mut u8).add(offset) as *mut bool }
    }

    pub fn in_prediction(&self) -> bool {
        unsafe { *self.flag(0xC) }
    }

    pub fn set_in_prediction(&self, value: bool) {
        unsafe { *self.flag(0xC) = value }
    }

    pub fn first_time_predicted(&self) -> bool {
        unsafe { *self.flag(0xD) }
    }

    pub fn set_first_time_predicted(&self, value: bool) {
        unsafe { *self.flag(0xD) = value }
    }

    pub fn engine_paused(&self) -> bool {
        unsafe { *self.flag(0xF) }
    }
}
