use std::ffi::{CStr, CString, c_char, c_void};

use crate::{
    interfaces::Interfaces,
    traits::FromRaw,
    types::{Material, RenderContext, Texture},
    vfunc,
};

#[derive(Default, Clone)]
pub struct MaterialSystem {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl MaterialSystem {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        MaterialSystem { this, vtable }
    }

    pub fn find_material(&self, name: &str) -> Option<Material> {
        let f = vfunc!(
            self.vtable,
            71,
            (*mut c_void, *const c_char, *const c_char, bool, *const c_char) -> *mut c_void
        );

        let c_name = CString::new(name).ok()?;
        let ptr = f(
            self.this,
            c_name.as_ptr(),
            c"Model textures".as_ptr(),
            true,
            std::ptr::null(),
        );
        if ptr.is_null() {
            return None;
        }

        Some(Material::from_raw(ptr))
    }

    /// Creates a material from VMT text, the material system takes a reference to it
    pub fn create_material(&self, name: &CStr, vmt: &CStr) -> Option<Material> {
        let f = vfunc!(
            self.vtable,
            70,
            (*mut c_void, *const c_char, *mut c_void) -> *mut c_void
        );

        let kv = Interfaces::key_values().parse_buffer(name, vmt)?;
        let ptr = f(self.this, name.as_ptr(), kv);
        if ptr.is_null() {
            return None;
        }

        Some(Material::from_raw(ptr))
    }

    /// The render target `name`, such as `_rt_FullFrameFB`
    pub fn find_texture(&self, name: &CStr) -> Option<Texture> {
        let f = vfunc!(
            self.vtable,
            79,
            (*mut c_void, *const c_char, *const c_char, bool, i32) -> *mut c_void
        );
        let ptr = f(self.this, name.as_ptr(), c"RenderTargets".as_ptr(), true, 0);
        (!ptr.is_null()).then(|| Texture::from_raw(ptr))
    }

    /// The render context to draw with, until it's dropped
    pub fn render_context(&self) -> Option<RenderContext> {
        let f = vfunc!(self.vtable, 98, (*mut c_void) -> *mut c_void);
        let ptr = f(self.this);
        (!ptr.is_null()).then(|| RenderContext::begin(ptr))
    }
}
