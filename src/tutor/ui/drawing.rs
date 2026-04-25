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

/// Splittable RNG mix constants (Murmur3 / SplitMix64-derived). Each
/// is co-prime to 2^64 so XOR-mixing them with a small index gives
/// a wide spread of seed states for adjacent patches.
const SEED_BASE: usize = 0xDEADBEEF01234567;
const ROW_MIX: usize = 0x9E3779B94517B397;
const COARSE_X_MIX: usize = 0xBF58476D1CE4E5B9;
const COARSE_Y_MIX: usize = 0x94D049BB133111EB;
const FINE_X_MIX: usize = 0xD6E8FEB86659FD93;
const FINE_Y_MIX: usize = 0xC2B2AE3D27D4EB4F;

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
    let row_seed: usize = SEED_BASE
        ^ ((logical_row as usize)
            .wrapping_sub(height / 2)
            .wrapping_mul(ROW_MIX));

    let mask = theme::BG_MASK;
    let alpha = theme::BG_ALPHA;
    let ones = 0x00010101u32;
    let base = theme::BG_BASE;
    let speckle_colour = theme::BG_SPECKLE;

    // Per-row vertical patch hashes — mixed in at horizontal patch
    // boundaries so the (cx, cy) cell as a whole gets its own seed.
    let dist_y = (logical_row.unsigned_abs() as usize).wrapping_sub(height / 2);
    let coarse_y = (dist_y / PATCH_COARSE).wrapping_mul(COARSE_Y_MIX);
    let fine_y = (dist_y / PATCH_FINE).wrapping_mul(FINE_Y_MIX);

    let centre = width / 2;

    // Right half: left-to-right.
    let mut rng = row_seed;
    let mut colour = rng as u32 & mask | alpha;
    let mut last_coarse = usize::MAX;
    let mut last_fine = usize::MAX;
    for x in centre..x_end {
        let dist_x = x - centre;
        let coarse = dist_x / PATCH_COARSE;
        let fine = dist_x / PATCH_FINE;
        if coarse != last_coarse {
            rng ^= coarse.wrapping_mul(COARSE_X_MIX) ^ coarse_y;
            last_coarse = coarse;
        }
        if fine != last_fine {
            rng ^= fine.wrapping_mul(FINE_X_MIX) ^ fine_y;
            last_fine = fine;
        }
        rng ^= rng.rotate_left(13).wrapping_add(12345678942);
        let adder = rng as u32 & ones;
        if rng.wrapping_add(speckle) < usize::MAX / 256 {
            colour = rng as u32 >> 8 & speckle_colour | alpha;
        } else {
            colour = colour.wrapping_add(adder) & mask;
            let subtractor = (rng >> 5) as u32 & ones;
            colour = colour.wrapping_sub(subtractor) & mask;
        }
        row_pixels[x] = colour.wrapping_add(base) | alpha;
    }

    // Left half: right-to-left, distance-from-centre indexed so the
    // patch-boundary mix-ins land at mirrored x's, preserving overall
    // left/right symmetry of the patchwork.
    rng = row_seed;
    colour = rng as u32 & mask | alpha;
    last_coarse = usize::MAX;
    last_fine = usize::MAX;
    for x in (x_start..centre).rev() {
        let dist_x = centre - 1 - x;
        let coarse = dist_x / PATCH_COARSE;
        let fine = dist_x / PATCH_FINE;
        if coarse != last_coarse {
            rng ^= coarse.wrapping_mul(COARSE_X_MIX) ^ coarse_y;
            last_coarse = coarse;
        }
        if fine != last_fine {
            rng ^= fine.wrapping_mul(FINE_X_MIX) ^ fine_y;
            last_fine = fine;
        }
        rng ^= rng.rotate_left(13).wrapping_sub(12345678942);
        let adder = rng as u32 & ones;
        if rng.wrapping_add(speckle) < usize::MAX / 256 {
            colour = rng as u32 >> 8 & speckle_colour | alpha;
        } else {
            colour = colour.wrapping_add(adder) & mask;
            let subtractor = (rng >> 5) as u32 & ones;
            colour = colour.wrapping_sub(subtractor) & mask;
        }
        row_pixels[x] = colour.wrapping_add(base) | alpha;
    }
}
