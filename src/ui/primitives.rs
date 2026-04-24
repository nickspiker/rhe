//! Minimal drawing primitives on a u32 ARGB pixel buffer.
//!
//! Tutor-only for now: solid fills, axis-aligned rectangles, and 1px
//! hairlines. Photon's antialiased pill-button + window-controls
//! primitives are intentionally NOT lifted here — they're 800+ lines
//! tangled with PhotonApp hit-test infrastructure, and the tutor
//! ships fine with rectangles. When we want prettier cells later we
//! can upgrade in place without touching callers.
//!
//! All coordinates and dimensions are expected to have been computed
//! upstream from `scale::span * ru / divisor`. The only pixel literal
//! allowed in this module is the 1 in `hline`/`vline` (the hairline
//! width).

/// Clear the entire buffer to a solid color.
pub fn fill(pixels: &mut [u32], color: u32) {
    for p in pixels.iter_mut() {
        *p = color;
    }
}

/// Fill an axis-aligned rectangle. Clips to buffer bounds.
pub fn fill_rect(
    pixels: &mut [u32],
    buf_width: i32,
    buf_height: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u32,
) {
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(buf_width);
    let y1 = (y + h).min(buf_height);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let stride = buf_width as usize;
    for row in y0..y1 {
        let base = row as usize * stride;
        for col in x0..x1 {
            pixels[base + col as usize] = color;
        }
    }
}

/// 1px horizontal line at row `y`, spanning `[x, x+len)`. Clipped.
pub fn hline(pixels: &mut [u32], buf_width: i32, buf_height: i32, x: i32, y: i32, len: i32, color: u32) {
    if y < 0 || y >= buf_height {
        return;
    }
    let x0 = x.max(0);
    let x1 = (x + len).min(buf_width);
    if x0 >= x1 {
        return;
    }
    let base = y as usize * buf_width as usize;
    for col in x0..x1 {
        pixels[base + col as usize] = color;
    }
}

/// 1px vertical line at column `x`, spanning `[y, y+len)`. Clipped.
pub fn vline(pixels: &mut [u32], buf_width: i32, buf_height: i32, x: i32, y: i32, len: i32, color: u32) {
    if x < 0 || x >= buf_width {
        return;
    }
    let y0 = y.max(0);
    let y1 = (y + len).min(buf_height);
    if y0 >= y1 {
        return;
    }
    let stride = buf_width as usize;
    for row in y0..y1 {
        pixels[row as usize * stride + x as usize] = color;
    }
}

/// Rectangle outline in 1px hairline (four sides).
pub fn stroke_rect(
    pixels: &mut [u32],
    buf_width: i32,
    buf_height: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u32,
) {
    hline(pixels, buf_width, buf_height, x, y, w, color);
    hline(pixels, buf_width, buf_height, x, y + h - 1, w, color);
    vline(pixels, buf_width, buf_height, x, y, h, color);
    vline(pixels, buf_width, buf_height, x + w - 1, y, h, color);
}

/// Pack 0xAARRGGBB from components (softbuffer native order on all
/// desktop targets). Alpha is top byte; typical usage passes 0xFF.
#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000u32 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Same but with explicit alpha.
#[inline]
pub const fn argb(a: u8, r: u8, g: u8, b: u8) -> u32 {
    ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}
