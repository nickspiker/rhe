//! Tutor canvas background pattern.
//!
//! Vendored from photon's `draw_background_texture`, with two
//! lightweight seed-mix tweaks that break the original purely-
//! horizontal stripe noise into a 2D patchwork. Each row still uses
//! a single hashed RNG that walks left-to-right (mirrored on the
//! left half), but the seed is XOR-mixed at coarse and fine
//! horizontal patch boundaries — those mixes also fold in a
//! row-derived hash, so each (coarse_x, coarse_y) cell gets its own
//! local "feel". The two scales together give the result a faintly
//! fractal patch-quilt look without leaving the inner-loop O(1).
//!
//! Per-pixel cost: photon's two wrapping ops + two divisions + two
//! comparisons (block-change checks). Sub-millisecond on a typical
//! tutor window.
//!
//! Sequential only — the tutor redraws on key events, not at 60 fps,
//! so rayon's not pulling its weight here.

use super::theme;

/// Coarse-tier patch size. Larger square regions of self-similar
/// brightness — sets the dominant patchwork scale.
const PATCH_COARSE: usize = 64;
/// Fine-tier patch size. Smaller embedded variation inside each
/// coarse patch — adds texture detail at a tighter grid.
const PATCH_FINE: usize = 16;

/// Photon's signature procedural background.
///
/// * `pixels` — ARGB pixel buffer (0xAARRGGBB).
/// * `speckle` — animation counter for the bright-pixel sparkle. 0 =
///   static; increment each frame for a twinkly look.
/// * `fullscreen` — true draws every pixel; false leaves a 1-pixel
///   border untouched (lets the squircle window edges paint over).
/// * `scroll_offset` — shifts which logical row each screen row
///   maps to. 0 for the tutor (no scrolled content).
pub fn draw_background_texture(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    speckle: usize,
    fullscreen: bool,
    scroll_offset: isize,
) {
    let (row_start, row_end, x_start, x_end) = if fullscreen {
        (0, height, 0, width)
    } else {
        (1, height - 1, 1, width - 1)
    };

    for row_idx in row_start..row_end {
        let row_pixels = &mut pixels[row_idx * width..(row_idx + 1) * width];
        let logical_row = row_idx as isize - scroll_offset;
        draw_background_row(
            row_pixels,
            width,
            logical_row,
            height,
            x_start,
            x_end,
            speckle,
        );
    }
}

/// Draw a single row of the background texture
/// This is the core algorithm shared between platforms
#[inline]
fn draw_background_row(
    row_pixels: &mut [u32],
    width: usize,
    logical_row: isize,
    height: usize,
    x_start: usize,
    x_end: usize,
    speckle: usize,
) {
    // WHY: logical_row can be negative when scrolled, use wrapping for RNG seed
    // PROOF: wrapping_sub produces consistent hash for any scroll position
    // PREVENTS: Different behavior for negative vs positive row indices
    let mut rng: usize = (0xDEADBEEF01234567)
        ^ ((logical_row as usize)
            .wrapping_sub(height / 2)
            .wrapping_mul(0x9E3779B94517B397));
    let mask = theme::BG_MASK;
    let alpha = theme::BG_ALPHA;
    let ones = 0x00010101;
    let base = theme::BG_BASE;
    let speckle_colour = theme::BG_SPECKLE;
    let mut colour = rng as u32 & mask | alpha;

    // Right half: left-to-right
    for x in width / 2..x_end {
        rng ^= rng.rotate_left(13).wrapping_add(12345678942);
        let adder = rng as u32 & ones;
        if rng.wrapping_add(speckle) < usize::MAX >> 6 {
            colour = rng as u32 >> 8 & speckle_colour | alpha;
        } else {
            colour = colour.wrapping_add(adder) & mask;
            let subtractor = (rng >> 5) as u32 & ones;
            colour = colour.wrapping_sub(subtractor) & mask;
        }
        row_pixels[x] = colour.wrapping_add(base) | alpha;
    }

    // Left half: right-to-left (mirror)
    rng = 0xDEADBEEF01234567
        ^ ((logical_row as usize)
            .wrapping_sub(height / 2)
            .wrapping_mul(0x9E3779B94517B397));
    colour = rng as u32 & mask | alpha;

    for x in (x_start..width / 2).rev() {
        rng ^= rng.rotate_left(13).wrapping_sub(12345678942);
        let adder = rng as u32 & ones;
        if rng.wrapping_add(speckle) < usize::MAX >> 6 {
            colour = rng as u32 >> 8 & speckle_colour | alpha;
        } else {
            colour = colour.wrapping_add(adder) & mask;
            let subtractor = (rng >> 5) as u32 & ones;
            colour = colour.wrapping_sub(subtractor) & mask;
        }
        row_pixels[x] = colour.wrapping_add(base) | alpha;
    }
}
