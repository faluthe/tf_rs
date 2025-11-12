use std::ffi::{CString, c_void};

use crate::vfunc;

#[derive(Default, Clone)]
pub struct Surface {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl Surface {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        Surface { this, vtable }
    }

    pub fn draw_set_color(&self, r: i32, g: i32, b: i32, a: i32) {
        let f = vfunc!(
            self.vtable,
            10,
            extern "C" fn(*mut c_void, i32, i32, i32, i32) -> ()
        );
        f(self.this, r, g, b, a)
    }

    pub fn draw_outlined_rect(&self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let f = vfunc!(
            self.vtable,
            14,
            extern "C" fn(*mut c_void, i32, i32, i32, i32) -> ()
        );
        f(self.this, x0, y0, x1, y1)
    }

    pub fn draw_set_text_font(&self, font: u64) {
        let f = vfunc!(self.vtable, 17, extern "C" fn(*mut c_void, u64) -> ());
        f(self.this, font)
    }

    pub fn draw_set_text_color(&self, r: i32, g: i32, b: i32, a: i32) {
        let f = vfunc!(
            self.vtable,
            18,
            extern "C" fn(*mut c_void, i32, i32, i32, i32) -> ()
        );
        f(self.this, r, g, b, a)
    }

    pub fn draw_set_text_pos(&self, x: u32, y: u32) {
        let f = vfunc!(self.vtable, 20, extern "C" fn(*mut c_void, u32, u32) -> ());
        f(self.this, x, y)
    }

    pub fn draw_print_text(&self, text: &str) {
        let f = vfunc!(
            self.vtable,
            22,
            extern "C" fn(*mut c_void, *const u32, i32, i32) -> ()
        );

        let wide: Vec<u32> = text
            .encode_utf16()
            .map(|c| c as u32)
            .chain(std::iter::once(0))
            .collect();
        f(self.this, wide.as_ptr(), wide.len() as i32, 0);
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        let f = vfunc!(self.vtable, 52, extern "C" fn(*mut c_void, bool) -> ());
        f(self.this, visible)
    }

    pub fn create_font(&self) -> u64 {
        let f = vfunc!(self.vtable, 66, extern "C" fn(*mut c_void) -> u64);
        f(self.this)
    }

    pub fn set_font_glyph_set(
        &self,
        font: u64,
        font_name: &str,
        tall: i32,
        weight: i32,
        blur: i32,
        scanlines: i32,
        flags: i32,
    ) -> bool {
        let f = vfunc!(
            self.vtable,
            67,
            extern "C" fn(*mut c_void, u64, *const i8, i32, i32, i32, i32, i32, i32, i32) -> bool
        );
        let c_font_name = CString::new(font_name).expect("Failed to convert font name to CString");
        f(
            self.this,
            font,
            c_font_name.as_ptr(),
            tall,
            weight,
            blur,
            scanlines,
            flags,
            0,
            0,
        )
    }
}
