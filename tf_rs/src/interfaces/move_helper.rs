use std::ffi::c_void;

use crate::{helpers::module, vfunc};

/// The client's IMoveHelper singleton, which game movement reports touches and sounds to
#[derive(Default, Clone)]
pub struct MoveHelper {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl MoveHelper {
    /// Find the static CMoveHelperClient instance through its vtable
    pub fn find() -> anyhow::Result<Self> {
        let segments = module::segments("client.so");
        let vtable = module::vtable_of(&segments, c"17CMoveHelperClient")
            .ok_or_else(|| anyhow::anyhow!("CMoveHelperClient vtable not found"))?;
        let this = module::find_object(&segments, vtable)
            .ok_or_else(|| anyhow::anyhow!("CMoveHelperClient instance not found"))?;
        Ok(MoveHelper {
            this: this as *mut c_void,
            vtable: vtable as *mut *mut c_void,
        })
    }

    pub fn this(&self) -> *mut c_void {
        self.this
    }

    pub fn is_valid(&self) -> bool {
        !self.this.is_null()
    }

    pub fn set_host(&self, player: *mut c_void) {
        let f = vfunc!(
            self.vtable,
            12,
            extern "C" fn(*mut c_void, *mut c_void) -> ()
        );
        f(self.this, player)
    }
}
