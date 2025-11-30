pub struct RGBA {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub a: i32,
}

impl RGBA {
    pub const RED: Self = Self {
        r: 157,
        g: 49,
        b: 47,
        a: 255,
    };
    pub const BLUE: Self = Self {
        r: 91,
        g: 122,
        b: 140,
        a: 255,
    };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
}
