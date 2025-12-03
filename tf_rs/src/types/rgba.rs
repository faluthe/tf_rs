pub struct RGBA {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub a: i32,
}

pub static RED: RGBA = RGBA {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};

pub static BLUE: RGBA = RGBA {
    r: 0,
    g: 0,
    b: 255,
    a: 255,
};

pub static WHITE: RGBA = RGBA {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

pub static ORANGE: RGBA = RGBA {
    r: 255,
    g: 165,
    b: 0,
    a: 255,
};
