#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Into<nuklear_sys::nk_rect> for Rect {
    fn into(self) -> nuklear_sys::nk_rect {
        unsafe { nuklear_sys::nk_rect(self.x, self.y, self.w, self.h) }
    }
}

impl From<nuklear_sys::nk_rect> for Rect {
    fn from(r: nuklear_sys::nk_rect) -> Self {
        Rect {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        }
    }
}
