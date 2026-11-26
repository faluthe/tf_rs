#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl From<Rect> for nuklear_sys::nk_rect {
    fn from(val: Rect) -> Self {
        unsafe { nuklear_sys::nk_rect(val.x, val.y, val.w, val.h) }
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
