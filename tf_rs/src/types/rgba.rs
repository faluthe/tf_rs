use crate::struct_with_serialize;

struct_with_serialize! {
    #[derive(Clone, Copy)]
    pub struct ColorF {
        pub r: f32,
        pub g: f32,
        pub b: f32,
        pub a: f32,
    }
}

impl Default for ColorF {
    fn default() -> Self {
        ColorF { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
    }
}

pub static RED: ColorF = ColorF {
    r: 0.8627,
    g: 0.1765,
    b: 0.1373,
    a: 1.0,
};

pub static BLUE: ColorF = ColorF {
    r: 0.1569,
    g: 0.4314,
    b: 0.9412,
    a: 1.0,
};

pub static WHITE: ColorF = ColorF {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

pub static ORANGE: ColorF = ColorF {
    r: 1.0,
    g: 0.6471,
    b: 0.0,
    a: 1.0,
};

pub static LIGHT_GREY: ColorF = ColorF {
    r: 0.1961,
    g: 0.1961,
    b: 0.1961,
    a: 1.0,
};

pub static DARK_GREY: ColorF = ColorF {
    r: 0.1373,
    g: 0.1373,
    b: 0.1373,
    a: 1.0,
};
