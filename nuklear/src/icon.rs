use std::io::Cursor;

use nuklear_sys::{
    GL_CLAMP_TO_EDGE, GL_LINEAR, GL_RGBA, GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER,
    GL_TEXTURE_MIN_FILTER, GL_TEXTURE_WRAP_S, GL_TEXTURE_WRAP_T, GL_UNSIGNED_BYTE, glBindTexture,
    glGenTextures, glTexImage2D, glTexParameteri, nk_image, nk_image_id,
};

/// Icons loaded from PNGs in `nuklear/assets/icons`. Only the alpha channel is used, the
/// icon is drawn white and tinted when drawn, so the source can be any color.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Circular arrow
    Refresh,
}

/// Width and height icons are scaled to at load, drawn 1:1 to stay crisp
pub(crate) const ICON_SIZE: u32 = 14;

impl Icon {
    pub(crate) const ALL: [Icon; 1] = [Icon::Refresh];

    fn png(self) -> &'static [u8] {
        match self {
            Icon::Refresh => include_bytes!("../assets/icons/refresh.png"),
        }
    }

    pub(crate) fn index(self) -> usize {
        Icon::ALL.iter().position(|i| *i == self).unwrap_or(0)
    }

    /// Decode, scale and upload the icon as a GL texture. Must run with the menu's GL context
    /// current. Returns None and logs nothing if the PNG can't be decoded, the icon just won't
    /// draw.
    pub(crate) fn load(self) -> Option<nk_image> {
        let (alpha, w, h) = decode_alpha(self.png())?;
        let alpha = resample(&alpha, w, h, ICON_SIZE);
        let rgba: Vec<u8> = alpha.iter().flat_map(|&a| [255, 255, 255, a]).collect();

        unsafe {
            let mut tex = 0;
            glGenTextures(1, &mut tex);
            glBindTexture(GL_TEXTURE_2D, tex);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR as i32);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR as i32);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE as i32);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE as i32);
            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA as i32,
                ICON_SIZE as i32,
                ICON_SIZE as i32,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                rgba.as_ptr() as *const _,
            );
            glBindTexture(GL_TEXTURE_2D, 0);
            Some(nk_image_id(tex as i32))
        }
    }
}

/// Alpha channel of a PNG as one byte per pixel, plus its width and height
fn decode_alpha(data: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut decoder = png::Decoder::new(Cursor::new(data));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;

    let channels = info.color_type.samples();
    let pixels = buf[..info.buffer_size()].chunks_exact(channels);
    let alpha = match info.color_type {
        png::ColorType::Rgba | png::ColorType::GrayscaleAlpha => {
            pixels.map(|px| px[channels - 1]).collect()
        }
        // No alpha channel, treat the whole image as opaque
        _ => pixels.map(|_| 255).collect(),
    };
    Some((alpha, info.width, info.height))
}

/// Area average `src` (w x h) down or up to `size` x `size`
fn resample(src: &[u8], w: u32, h: u32, size: u32) -> Vec<u8> {
    let sx = w as f32 / size as f32;
    let sy = h as f32 / size as f32;
    let mut out = Vec::with_capacity((size * size) as usize);

    for oy in 0..size {
        let (y0, y1) = (oy as f32 * sy, (oy + 1) as f32 * sy);
        for ox in 0..size {
            let (x0, x1) = (ox as f32 * sx, (ox + 1) as f32 * sx);
            let mut sum = 0.0;
            let mut weight = 0.0;
            for y in y0.floor() as u32..(y1.ceil() as u32).min(h) {
                let wy = (y1.min(y as f32 + 1.0) - y0.max(y as f32)).max(0.0);
                for x in x0.floor() as u32..(x1.ceil() as u32).min(w) {
                    let wx = (x1.min(x as f32 + 1.0) - x0.max(x as f32)).max(0.0);
                    sum += src[(y * w + x) as usize] as f32 * wx * wy;
                    weight += wx * wy;
                }
            }
            out.push(if weight > 0.0 { (sum / weight).round() as u8 } else { 0 });
        }
    }
    out
}
