//! Photon's chrome/button/textbox drawing primitives, lifted verbatim
//! via cp from photon/src/ui/compositing.rs. The only edits are at the
//! top of the file — import paths adapted to rhe's module tree, the
//! render() state machine and other app-specific methods stripped, a
//! zero-sized PhotonApp type introduced so the impl block compiles.
//! Everything below the impl block is photon code unchanged.

use crate::ui::theme;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
const PREMULTIPLIED: bool = true;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
const PREMULTIPLIED: bool = false;

// Hit-test IDs — copied from photon/src/ui/app.rs so the draw fns stay
// byte-identical to photon. rhe only cares about the chrome IDs.
pub const HIT_NONE: u8 = 0;
pub const HIT_MINIMIZE_BUTTON: u8 = 1;
pub const HIT_MAXIMIZE_BUTTON: u8 = 2;
pub const HIT_CLOSE_BUTTON: u8 = 3;
pub const HIT_HANDLE_TEXTBOX: u8 = 4;
pub const HIT_PRIMARY_BUTTON: u8 = 5;
pub const HIT_AVATAR: u8 = 7;

/// Zero-sized carrier so photon's static `impl` methods keep their
/// `PhotonApp::` call prefix. No state — all methods below are static.
pub struct PhotonApp;

impl PhotonApp {

    /// Calculate window control bounds without drawing.
    /// Returns (start, crossings, button_x_start, button_height) needed for edges/hairlines.
    pub fn window_controls_bounds(
        window_width: u32,
        window_height: u32,
        ru: f32,
    ) -> (usize, Vec<(u16, u8, u8)>, usize, usize) {
        let window_width = window_width as usize;
        let window_height = window_height as usize;

        // Calculate button dimensions using harmonic mean (span) scaled by ru
        let span = 2.0 * window_width as f32 * window_height as f32
            / (window_width as f32 + window_height as f32);
        let button_height = (span / 32.0 * ru).ceil() as usize;
        let button_width = button_height;
        let total_width = button_width * 7 / 2;

        let x_start = window_width - total_width;

        // Build squircle crossings for bottom-left corner
        let radius = span * ru / 4.;
        let squirdleyness = 24;

        let mut crossings: Vec<(u16, u8, u8)> = Vec::new();
        let mut y = 1f32;
        loop {
            let y_norm = y / radius;
            let x_norm = (1.0 - y_norm.powi(squirdleyness)).powf(1.0 / squirdleyness as f32);
            let x = x_norm * radius;
            let inset = radius - x;
            if inset > 0. {
                crossings.push((
                    inset as u16,
                    (inset.fract().sqrt() * 256.) as u8,
                    ((1. - inset.fract()).sqrt() * 256.) as u8,
                ));
            }
            if x < y {
                break;
            }
            y += 1.;
        }
        let start = (radius - y) as usize;
        let crossings: Vec<(u16, u8, u8)> = crossings.into_iter().rev().collect();

        // Return button_x_start with the offset applied (matches draw_window_controls)
        (start, crossings, x_start + button_width / 4, button_height)
    }

    pub fn draw_window_controls(
        pixels: &mut [u32],
        hit_test_map: &mut [u8],
        window_width: u32,
        window_height: u32,
        ru: f32,
    ) -> (usize, Vec<(u16, u8, u8)>, usize, usize) {
        let window_width = window_width as usize;
        let window_height = window_height as usize;

        // Calculate button dimensions using harmonic mean (span) scaled by ru
        // span = 2wh/(w+h), base button size = span/32, scaled by ru (zoom multiplier)
        let span = 2.0 * window_width as f32 * window_height as f32
            / (window_width as f32 + window_height as f32);
        let button_height = (span / 32.0 * ru).ceil() as usize;
        let button_width = button_height;
        let total_width = button_width * 7 / 2;

        // Buttons extend to top-right corner of window
        let mut x_start = window_width - total_width;
        let y_start = 0;

        // Build squircle crossings for bottom-left corner
        let radius = span * ru / 4.;
        let squirdleyness = 24;

        let mut crossings: Vec<(u16, u8, u8)> = Vec::new();
        let mut y = 1f32;
        loop {
            let y_norm = y / radius;
            let x_norm = (1.0 - y_norm.powi(squirdleyness)).powf(1.0 / squirdleyness as f32);
            let x = x_norm * radius;
            let inset = radius - x;
            if inset > 0. {
                crossings.push((
                    inset as u16,
                    (inset.fract().sqrt() * 256.) as u8,
                    ((1. - inset.fract()).sqrt() * 256.) as u8,
                ));
            }
            if x < y {
                break;
            }
            y += 1.;
        }
        let start = (radius - y) as usize;
        let crossings: Vec<(u16, u8, u8)> = crossings.into_iter().rev().collect();

        let edge_colour = theme::WINDOW_LIGHT_EDGE;
        let bg_colour = theme::WINDOW_CONTROLS_BG;

        // Left edge (vertical) - draw light hairline following squircle curve
        let mut y_offset = start;
        for (inset, l, h) in &crossings {
            if y_offset >= button_height {
                break;
            }
            let py = y_start + button_height - 1 - y_offset;

            // Fill grey to the right of the curve and populate hit test map
            let col_end = total_width.min(window_width - x_start);
            for col in (*inset as usize + 2)..col_end - 1 {
                let px = x_start + col;
                let pixel_idx = (py * window_width + px) as usize;

                // Write packed ARGB colour directly
                pixels[pixel_idx] = bg_colour;

                // Determine which button this pixel belongs to
                // Button widths: minimize (0-1), maximize (1-2), close (2-3.5)
                // Buttons are drawn with a button_width / 4 offset
                let button_area_x_start = x_start + button_width / 4;

                // Determine button ID based on x position
                // Handle the case where px might be before button_area_x_start
                let button_id = if px < button_area_x_start {
                    HIT_MINIMIZE_BUTTON // Left edge before offset belongs to minimize
                } else {
                    let x_in_button_area = px - button_area_x_start;
                    if x_in_button_area < button_width {
                        HIT_MINIMIZE_BUTTON
                    } else if x_in_button_area < button_width * 2 {
                        HIT_MAXIMIZE_BUTTON
                    } else {
                        HIT_CLOSE_BUTTON
                    }
                };
                hit_test_map[pixel_idx] = button_id;
            }

            let px = x_start + *inset as usize;
            let pixel_idx = (py * window_width + px) as usize;
            pixels[pixel_idx] = blend_rgb_only(pixels[pixel_idx], edge_colour, *l, *h);

            let px = x_start + *inset as usize + 1;
            let pixel_idx = (py * window_width + px) as usize;
            pixels[pixel_idx] = blend_rgb_only(bg_colour, edge_colour, *h, *l);

            // Populate hit test map for inner edge pixel
            let button_area_x_start = x_start + button_width / 4;

            let button_id = if px < button_area_x_start {
                HIT_MINIMIZE_BUTTON
            } else {
                let x_in_button_area = px - button_area_x_start;
                if x_in_button_area < button_width {
                    HIT_MINIMIZE_BUTTON
                } else if x_in_button_area < button_width * 2 {
                    HIT_MAXIMIZE_BUTTON
                } else {
                    HIT_CLOSE_BUTTON
                }
            };
            hit_test_map[pixel_idx] = button_id;

            y_offset += 1;
        }

        // Bottom edge (horizontal)
        let mut x_offset = start;
        let crossing_limit = crossings.len().min(window_width - (x_start + start));
        for &(inset, l, h) in &crossings[..crossing_limit] {
            let i = inset as usize;
            let px = x_start + x_offset;

            // Outer edge pixel (blend hairline with background texture behind)
            let py = y_start + button_height - 1 - i;
            let pixel_idx = py * window_width + px;
            pixels[pixel_idx] = blend_rgb_only(pixels[pixel_idx], edge_colour, l, h);

            // Fill grey above the curve (towards center of buttons) and populate hit test
            for row in (i + 2)..start {
                let py = y_start + button_height - 1 - row;
                let pixel_idx = py * window_width + px;

                pixels[pixel_idx] = bg_colour;

                // Determine which button this pixel belongs to
                // Buttons are drawn with a button_width / 4 offset
                let button_area_x_start = x_start + button_width / 4;

                // Handle the case where px might be before button_area_x_start
                let button_id = if px < button_area_x_start {
                    HIT_MINIMIZE_BUTTON // Left edge before offset belongs to minimize
                } else {
                    let x_in_button_area = px - button_area_x_start;
                    if x_in_button_area < button_width {
                        HIT_MINIMIZE_BUTTON
                    } else if x_in_button_area < button_width * 2 {
                        HIT_MAXIMIZE_BUTTON
                    } else {
                        HIT_CLOSE_BUTTON
                    }
                };
                hit_test_map[pixel_idx] = button_id;
            }

            let py = y_start + button_height - 1 - (i + 1);
            let pixel_idx = py * window_width + px;
            pixels[pixel_idx] = blend_rgb_only(bg_colour, edge_colour, h, l);

            // Populate hit test map for inner edge pixel
            let button_area_x_start = x_start + button_width / 4;

            let button_id = if px < button_area_x_start {
                HIT_MINIMIZE_BUTTON
            } else {
                let x_in_button_area = px - button_area_x_start;
                if x_in_button_area < button_width {
                    HIT_MINIMIZE_BUTTON
                } else if x_in_button_area < button_width * 2 {
                    HIT_MAXIMIZE_BUTTON
                } else {
                    HIT_CLOSE_BUTTON
                }
            };
            hit_test_map[pixel_idx] = button_id;

            x_offset += 1;
        }

        // Continue bottom edge linearly from where squircle ends to window edge
        let linear_start_x = x_start + start + crossings.len();
        let edge_y = y_start + button_height - 1;

        for px in linear_start_x..window_width {
            // Draw edge pixel at bottom of button area
            let pixel_idx = edge_y * window_width + px;
            pixels[pixel_idx] = edge_colour;

            // Fill grey above the edge (from edge to top of button area)
            for row in 1..start {
                let py = edge_y - row;
                let pixel_idx = py * window_width + px;
                pixels[pixel_idx] = bg_colour;

                // All pixels past the squircle belong to close button
                hit_test_map[pixel_idx] = HIT_CLOSE_BUTTON;
            }
        }

        x_start += button_width / 4;

        // Draw button symbols using glyph colours
        let (r, g, b, _a) = unpack_argb(theme::MINIMIZE_GLYPH);
        let minimize_colour = (r, g, b);
        Self::draw_minimize_symbol(
            pixels,
            window_width,
            x_start + button_width / 2,
            y_start + button_width / 2,
            button_width / 4,
            minimize_colour,
        );

        let (r, g, b, _a) = unpack_argb(theme::MAXIMIZE_GLYPH);
        let maximize_colour = (r, g, b);
        let (r, g, b, _a) = unpack_argb(theme::MAXIMIZE_GLYPH_INTERIOR);
        let maximize_interior = (r, g, b);
        Self::draw_maximize_symbol(
            pixels,
            window_width,
            x_start + button_width + button_width / 2,
            y_start + button_width / 2,
            button_width / 4,
            maximize_colour,
            maximize_interior,
        );

        let (r, g, b, _a) = unpack_argb(theme::CLOSE_GLYPH);
        let close_colour = (r, g, b);
        Self::draw_close_symbol(
            pixels,
            window_width,
            x_start + button_width * 2 + button_width / 2,
            y_start + button_width / 2,
            button_width / 4,
            close_colour,
        );
        (start, crossings, x_start, button_height)
    }

    pub fn draw_minimize_symbol(
        pixels: &mut [u32],
        width: usize,
        x: usize,
        y: usize,
        r: usize,
        stroke_colour: (u8, u8, u8),
    ) {
        let r = r + 1;
        let r_render = r / 4 + 1;
        let r_2 = r_render * r_render;
        let r_4 = r_2 * r_2;
        let r_3 = r_render * r_render * r_render;

        let stroke_packed = pack_argb(stroke_colour.0, stroke_colour.1, stroke_colour.2, 255);

        for h in -(r_render as isize)..=(r_render as isize) {
            for w in -(r as isize)..=(r as isize) {
                // Regular squircle: h^4 + w^4
                let h2 = h * h;
                let h4 = h2 * h2;
                let a = (w.abs() - (r * 3 / 4) as isize).max(0);
                let w2 = a * a;
                let w4 = w2 * w2;
                let dist_4 = (h4 + w4) as usize;

                if dist_4 <= r_4 {
                    let px = (x as isize + w) as usize;
                    let py = (y as isize + h + (r / 2) as isize) as usize;
                    let idx = py * width + px;
                    let gradient = ((r_4 - dist_4) << 8) / (r_3 << 2);
                    if gradient > 255 {
                        pixels[idx] = stroke_packed;
                    } else {
                        // Blend background towards stroke_colour using packed SIMD
                        let alpha = gradient as u64;
                        let inv_alpha = 256 - alpha;

                        // Widen bg pixel to packed channels
                        let mut bg = pixels[idx] as u64;
                        bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
                        bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

                        // Widen stroke colour to packed channels
                        let mut stroke = stroke_packed as u64;
                        stroke = (stroke | (stroke << 16)) & 0x0000FFFF0000FFFF;
                        stroke = (stroke | (stroke << 8)) & 0x00FF00FF00FF00FF;

                        // Blend: bg * inv_alpha + stroke * alpha
                        let mut blended = bg * inv_alpha + stroke * alpha;

                        // Contract back to u32
                        blended = (blended >> 8) & 0x00FF00FF00FF00FF;
                        blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
                        blended = blended | (blended >> 16);
                        pixels[idx] = blended as u32;
                    }
                }
            }
        }
    }

    pub fn draw_maximize_symbol(
        pixels: &mut [u32],
        width: usize,
        x: usize,
        y: usize,
        r: usize,
        stroke_colour: (u8, u8, u8),
        fill_colour: (u8, u8, u8),
    ) {
        let r = r + 1;
        let mut r_4 = r * r;
        r_4 *= r_4;
        let r_3 = r * r * r;

        // Inner radius (inset by r/6)
        let r_inner = r * 4 / 5;
        let mut r_inner_4 = r_inner * r_inner;
        r_inner_4 *= r_inner_4;
        let r_inner_3 = r_inner * r_inner * r_inner;

        // Edge threshold: gradient spans approximately 4r^3 worth of dist_4 change
        let outer_edge_threshold = r_3 << 2;
        let inner_edge_threshold = r_inner_3 << 2;

        let stroke_packed = pack_argb(stroke_colour.0, stroke_colour.1, stroke_colour.2, 255);
        let fill_packed = pack_argb(fill_colour.0, fill_colour.1, fill_colour.2, 255);

        for h in -(r as isize)..=r as isize {
            for w in -(r as isize)..=r as isize {
                let h2 = h * h;
                let h4 = h2 * h2;
                let w2 = w * w;
                let w4 = w2 * w2;
                let dist_4 = (h4 + w4) as usize;

                if dist_4 <= r_4 {
                    let px = (x as isize + w) as usize;
                    let py = (y as isize + h) as usize;
                    let idx = py * width + px;

                    // Determine which zone we're in
                    let dist_from_outer = r_4 - dist_4;

                    if dist_4 <= r_inner_4 {
                        let dist_from_inner = r_inner_4 - dist_4;

                        // Inside inner squircle
                        if dist_from_inner <= inner_edge_threshold {
                            // Inner edge: blend from stroke to fill using packed SIMD
                            let gradient = ((dist_from_inner) << 8) / inner_edge_threshold;
                            let alpha = gradient as u64;
                            let inv_alpha = 256 - alpha;

                            let mut stroke = stroke_packed as u64;
                            stroke = (stroke | (stroke << 16)) & 0x0000FFFF0000FFFF;
                            stroke = (stroke | (stroke << 8)) & 0x00FF00FF00FF00FF;

                            let mut fill = fill_packed as u64;
                            fill = (fill | (fill << 16)) & 0x0000FFFF0000FFFF;
                            fill = (fill | (fill << 8)) & 0x00FF00FF00FF00FF;

                            let mut blended = stroke * inv_alpha + fill * alpha;
                            blended = (blended >> 8) & 0x00FF00FF00FF00FF;
                            blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
                            blended = blended | (blended >> 16);
                            pixels[idx] = blended as u32;
                        } else {
                            // Solid fill center
                            pixels[idx] = fill_packed;
                        }
                    } else {
                        // Between inner and outer: stroke ring
                        if dist_from_outer <= outer_edge_threshold {
                            // Outer edge: blend from background to stroke using packed SIMD
                            let gradient = ((dist_from_outer) << 8) / outer_edge_threshold;
                            let alpha = gradient as u64;
                            let inv_alpha = 256 - alpha;

                            let mut bg = pixels[idx] as u64;
                            bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
                            bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

                            let mut stroke = stroke_packed as u64;
                            stroke = (stroke | (stroke << 16)) & 0x0000FFFF0000FFFF;
                            stroke = (stroke | (stroke << 8)) & 0x00FF00FF00FF00FF;

                            let mut blended = bg * inv_alpha + stroke * alpha;
                            blended = (blended >> 8) & 0x00FF00FF00FF00FF;
                            blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
                            blended = blended | (blended >> 16);
                            pixels[idx] = blended as u32;
                        } else {
                            // Solid stroke ring
                            pixels[idx] = stroke_packed;
                        }
                    }
                }
            }
        }
    }

    pub fn draw_close_symbol(
        pixels: &mut [u32],
        width: usize,
        x: usize,
        y: usize,
        r: usize,
        stroke_colour: (u8, u8, u8),
    ) {
        let r = r + 1;
        // Draw X with antialiased rounded-end diagonals (capsule/pill shaped)
        let thickness = (r / 3).max(1) as f32;
        let radius = thickness / 2.;
        let size = (r * 2) as f32; // X spans diameter, not radius
        let cxf = x as f32;
        let cyf = y as f32;

        let end = size / 3.;

        // Define the two diagonal line segments
        // Diagonal 1: top-left to bottom-right
        let x1_start = cxf - end;
        let y1_start = cyf - end;
        let x1_end = cxf + end;
        let y1_end = cyf + end;

        // Diagonal 2: top-right to bottom-left
        let x2_start = cxf + end;
        let y2_start = cyf - end;
        let x2_end = cxf - end;
        let y2_end = cyf + end;

        // Pack stroke colour once
        let stroke_packed = pack_argb(stroke_colour.0, stroke_colour.1, stroke_colour.2, 255);

        // Scan the bounding box and render both capsules
        let min_x = ((x as i32) - (r as i32)).max(0);
        let max_x = ((x as i32) + (r as i32)).min(width as i32);
        let min_y = ((y as i32) - (r as i32)).max(0);
        let max_y = ((y as i32) + (r as i32)).min(width as i32);

        let cxi = x as i32;
        let cyi = y as i32;

        // Quadrant 1: top-left (diagonal 1)
        for py in min_y..cyi {
            for px in min_x..cxi {
                let px_f = px as f32 + 0.5;
                let py_f = py as f32 + 0.5;

                let dist = Self::distance_to_capsule(
                    px_f, py_f, x1_start, y1_start, x1_end, y1_end, radius,
                );

                let alpha_f = if dist < -0.5 {
                    1.
                } else if dist < 0.5 {
                    0.5 - dist
                } else {
                    0.
                };

                if alpha_f > 0. {
                    let idx = py as usize * width + px as usize;
                    let alpha = (alpha_f * 256.0) as u64;
                    let inv_alpha = 256 - alpha;

                    let mut bg = pixels[idx] as u64;
                    bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
                    bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

                    let mut stroke = stroke_packed as u64;
                    stroke = (stroke | (stroke << 16)) & 0x0000FFFF0000FFFF;
                    stroke = (stroke | (stroke << 8)) & 0x00FF00FF00FF00FF;

                    let mut blended = bg * inv_alpha + stroke * alpha;
                    blended = (blended >> 8) & 0x00FF00FF00FF00FF;
                    blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
                    blended = blended | (blended >> 16);
                    pixels[idx] = blended as u32;
                }
            }
        }

        // Quadrant 2: top-right (diagonal 2)
        for py in min_y..cyi {
            for px in cxi..max_x {
                let px_f = px as f32 + 0.5;
                let py_f = py as f32 + 0.5;

                let dist = Self::distance_to_capsule(
                    px_f, py_f, x2_start, y2_start, x2_end, y2_end, radius,
                );

                let alpha_f = if dist < -0.5 {
                    1.
                } else if dist < 0.5 {
                    0.5 - dist
                } else {
                    0.
                };

                if alpha_f > 0. {
                    let idx = py as usize * width + px as usize;
                    let alpha = (alpha_f * 256.0) as u64;
                    let inv_alpha = 256 - alpha;

                    let mut bg = pixels[idx] as u64;
                    bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
                    bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

                    let mut stroke = stroke_packed as u64;
                    stroke = (stroke | (stroke << 16)) & 0x0000FFFF0000FFFF;
                    stroke = (stroke | (stroke << 8)) & 0x00FF00FF00FF00FF;

                    let mut blended = bg * inv_alpha + stroke * alpha;
                    blended = (blended >> 8) & 0x00FF00FF00FF00FF;
                    blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
                    blended = blended | (blended >> 16);
                    pixels[idx] = blended as u32;
                }
            }
        }

        // Quadrant 3: bottom-left (diagonal 2)
        for py in cyi..max_y {
            for px in min_x..cxi {
                let px_f = px as f32 + 0.5;
                let py_f = py as f32 + 0.5;

                let dist = Self::distance_to_capsule(
                    px_f, py_f, x2_start, y2_start, x2_end, y2_end, radius,
                );

                let alpha_f = if dist < -0.5 {
                    1.
                } else if dist < 0.5 {
                    0.5 - dist
                } else {
                    0.
                };

                if alpha_f > 0. {
                    let idx = py as usize * width + px as usize;
                    let alpha = (alpha_f * 256.0) as u64;
                    let inv_alpha = 256 - alpha;

                    let mut bg = pixels[idx] as u64;
                    bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
                    bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

                    let mut stroke = stroke_packed as u64;
                    stroke = (stroke | (stroke << 16)) & 0x0000FFFF0000FFFF;
                    stroke = (stroke | (stroke << 8)) & 0x00FF00FF00FF00FF;

                    let mut blended = bg * inv_alpha + stroke * alpha;
                    blended = (blended >> 8) & 0x00FF00FF00FF00FF;
                    blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
                    blended = blended | (blended >> 16);
                    pixels[idx] = blended as u32;
                }
            }
        }

        // Quadrant 4: bottom-right (diagonal 1)
        for py in cyi..max_y {
            for px in cxi..max_x {
                let px_f = px as f32 + 0.5;
                let py_f = py as f32 + 0.5;

                let dist = Self::distance_to_capsule(
                    px_f, py_f, x1_start, y1_start, x1_end, y1_end, radius,
                );

                let alpha_f = if dist < -0.5 {
                    1.
                } else if dist < 0.5 {
                    0.5 - dist
                } else {
                    0.
                };

                if alpha_f > 0. {
                    let idx = py as usize * width + px as usize;
                    let alpha = (alpha_f * 256.0) as u64;
                    let inv_alpha = 256 - alpha;

                    let mut bg = pixels[idx] as u64;
                    bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
                    bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

                    let mut stroke = stroke_packed as u64;
                    stroke = (stroke | (stroke << 16)) & 0x0000FFFF0000FFFF;
                    stroke = (stroke | (stroke << 8)) & 0x00FF00FF00FF00FF;

                    let mut blended = bg * inv_alpha + stroke * alpha;
                    blended = (blended >> 8) & 0x00FF00FF00FF00FF;
                    blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
                    blended = blended | (blended >> 16);
                    pixels[idx] = blended as u32;
                }
            }
        }
    }

    /// Draw magnifying glass icon (circle with diagonal handle)
    pub fn draw_plus_symbol(
        pixels: &mut [u32],
        width: usize,
        cx: usize,
        cy: usize,
        size: usize,
        stroke_colour: (u8, u8, u8),
    ) {
        let scale = size as f32 / 1000.0;
        let stroke_width = 120.0 * scale; // Slightly thicker than magnify
        let radius = stroke_width / 2.0;
        let arm_length = 350.0 * scale; // Length from center to end

        let cxf = cx as f32;
        let cyf = cy as f32;

        // Horizontal bar endpoints
        let h_start_x = cxf - arm_length;
        let h_end_x = cxf + arm_length;

        // Vertical bar endpoints
        let v_start_y = cyf - arm_length;
        let v_end_y = cyf + arm_length;

        let stroke_packed = pack_argb(stroke_colour.0, stroke_colour.1, stroke_colour.2, 255);

        // Bounding box
        let half_size = (size / 2 + 2) as isize;
        let min_x = (cx as isize - half_size).max(0) as usize;
        let max_x = (cx as isize + half_size).min(width as isize) as usize;
        let min_y = (cy as isize - half_size).max(0) as usize;
        let max_y = (cy as isize + half_size).min((pixels.len() / width) as isize) as usize;

        for py in min_y..max_y {
            for px in min_x..max_x {
                let px_f = px as f32 + 0.5;
                let py_f = py as f32 + 0.5;

                // Distance to horizontal capsule
                let dist_h =
                    Self::distance_to_capsule(px_f, py_f, h_start_x, cyf, h_end_x, cyf, radius);

                // Distance to vertical capsule
                let dist_v =
                    Self::distance_to_capsule(px_f, py_f, cxf, v_start_y, cxf, v_end_y, radius);

                // Use minimum distance (union of shapes)
                let dist = dist_h.min(dist_v);

                // Antialiased rendering
                let alpha_f = if dist < -0.5 {
                    1.0
                } else if dist < 0.5 {
                    0.5 - dist
                } else {
                    0.0
                };

                if alpha_f > 0.0 {
                    let idx = py * width + px;
                    let alpha = (alpha_f * 256.0) as u64;
                    let inv_alpha = 256 - alpha;

                    let mut bg = pixels[idx] as u64;
                    bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
                    bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

                    let mut stroke = stroke_packed as u64;
                    stroke = (stroke | (stroke << 16)) & 0x0000FFFF0000FFFF;
                    stroke = (stroke | (stroke << 8)) & 0x00FF00FF00FF00FF;

                    let mut blended = bg * inv_alpha + stroke * alpha;
                    blended = (blended >> 8) & 0x00FF00FF00FF00FF;
                    blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
                    blended = blended | (blended >> 16);
                    pixels[idx] = blended as u32;
                }
            }
        }
    }

    /// Draw hourglass icon (two triangles meeting at center point)
    /// angle_degrees: rotation angle in degrees (stochastic wobble during search)
    fn distance_to_capsule_local(
        px: f32,
        py: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        radius: f32,
    ) -> f32 {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len_sq = dx * dx + dy * dy;

        let t = if len_sq > 0.0 {
            ((px - x1) * dx + (py - y1) * dy) / len_sq
        } else {
            0.0
        };
        let t = t.clamp(0.0, 1.0);

        let closest_x = x1 + t * dx;
        let closest_y = y1 + t * dy;
        let dist_x = px - closest_x;
        let dist_y = py - closest_y;

        (dist_x * dist_x + dist_y * dist_y).sqrt() - radius
    }

    // Helper function: distance from point to capsule (line segment with rounded ends)
    pub fn distance_to_capsule(
        px: f32,
        py: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        radius: f32,
    ) -> f32 {
        // Vector from start to end
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len_sq = dx * dx + dy * dy;

        // Project point onto line segment (clamped to [0, 1])
        let t = ((px - x1) * dx + (py - y1) * dy) / len_sq;
        let t_clamped = t.clamp(0., 1.);

        // Closest point on line segment
        let closest_x = x1 + t_clamped * dx;
        let closest_y = y1 + t_clamped * dy;

        // Distance to closest point minus radius
        let dist_x = px - closest_x;
        let dist_y = py - closest_y;
        (dist_x * dist_x + dist_y * dist_y).sqrt() - radius
    }

    // Motion triggered by network action, motion speed dependent on latency
    // Delegates to shared drawing module
    pub fn draw_window_edges_and_mask(
        pixels: &mut [u32],
        hit_test_map: &mut [u8],
        width: u32,
        height: u32,
        start: usize,
        crossings: &[(u16, u8, u8)],
    ) {
        let light_colour = theme::WINDOW_LIGHT_EDGE;
        let shadow_colour = theme::WINDOW_SHADOW_EDGE;

        // Fill all four edges with white before squircle clipping
        // Top edge
        for x in 0..width {
            let idx = 0 * width + x;
            pixels[idx as usize] = light_colour;
        }

        // Bottom edge
        for x in 0..width {
            let idx = (height - 1) * width + x;
            pixels[idx as usize] = shadow_colour;
        }

        // Left edge
        for y in 0..height {
            let idx = y * width + 0;
            pixels[idx as usize] = light_colour;
        }

        // Right edge
        for y in 0..height {
            let idx = y * width + (width - 1);
            pixels[idx as usize] = shadow_colour;
        }

        // Fill four corner squares and clear hitmap
        for row in 0..start {
            for col in 0..start {
                let idx = row * width as usize + col;
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }
        }
        for row in 0..start {
            for col in (width as usize - start)..width as usize {
                let idx = row * width as usize + col;
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }
        }
        for row in (height as usize - start)..height as usize {
            for col in 0..start {
                let idx = row * width as usize + col;
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }
        }
        for row in (height as usize - start)..height as usize {
            for col in (width as usize - start)..width as usize {
                let idx = row * width as usize + col;
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }
        }

        // Top left/right edges
        let mut y_top = start;
        for crossing in 0..crossings.len() {
            let (inset, l, h) = crossings[crossing];
            // Left edge fill
            for idx in y_top * width as usize..y_top * width as usize + inset as usize {
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }

            // Left edge outer pixel
            let pixel_idx = y_top * width as usize + inset as usize;
            if PREMULTIPLIED {
                pixels[pixel_idx] = scale_alpha(light_colour, h);
            } else {
                pixels[pixel_idx] = (light_colour & 0x00FFFFFF) | ((h as u32) << 24);
            }
            if h < 255 {
                hit_test_map[pixel_idx] = HIT_NONE; // NEEDS FIXED!!!
            }

            // Left edge inner pixel
            let pixel_idx = pixel_idx + 1;
            pixels[pixel_idx] = blend_rgb_only(pixels[pixel_idx], light_colour, h, l);

            // Right edge inner pixel
            let pixel_idx = y_top * width as usize + width as usize - 2 - inset as usize;
            pixels[pixel_idx] = blend_rgb_only(pixels[pixel_idx], shadow_colour, h, l);

            // Right edge outer pixel
            let pixel_idx = pixel_idx + 1;
            if PREMULTIPLIED {
                pixels[pixel_idx] = scale_alpha(shadow_colour, h);
            } else {
                pixels[pixel_idx] = (shadow_colour & 0x00FFFFFF) | ((h as u32) << 24);
            }
            if h < 255 {
                hit_test_map[pixel_idx] = HIT_NONE;
            }

            // Right edge fill
            for idx in (y_top * width as usize + width as usize - inset as usize)
                ..((y_top + 1) * width as usize)
            {
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }
            y_top += 1;
        }

        // Bottom left/right edges
        let mut y_bottom = height as usize - start - 1;
        for crossing in 0..crossings.len() {
            let (inset, l, h) = crossings[crossing];

            // Left edge fill
            for idx in y_bottom * width as usize..y_bottom * width as usize + inset as usize {
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }

            // Left outer edge pixel
            let pixel_idx = y_bottom * width as usize + inset as usize;
            if PREMULTIPLIED {
                pixels[pixel_idx] = scale_alpha(light_colour, h);
            } else {
                pixels[pixel_idx] = (light_colour & 0x00FFFFFF) | ((h as u32) << 24);
            }
            if h < 255 {
                hit_test_map[pixel_idx] = HIT_NONE;
            }

            // Left inner edge pixel
            let pixel_idx = pixel_idx + 1;
            pixels[pixel_idx] = blend_rgb_only(pixels[pixel_idx], light_colour, h, l);

            // Right edge inner pixel
            let pixel_idx = y_bottom * width as usize + width as usize - 2 - inset as usize;
            pixels[pixel_idx] = blend_rgb_only(pixels[pixel_idx], shadow_colour, h, l);

            // Right edge outer pixel
            let pixel_idx = pixel_idx + 1;
            if PREMULTIPLIED {
                pixels[pixel_idx] = scale_alpha(shadow_colour, h);
            } else {
                pixels[pixel_idx] = (shadow_colour & 0x00FFFFFF) | ((h as u32) << 24);
            }
            if h < 255 {
                hit_test_map[pixel_idx] = HIT_NONE;
            }

            // Right edge fill
            for idx in (y_bottom * width as usize + width as usize - inset as usize)
                ..((y_bottom + 1) * width as usize)
            {
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }

            y_bottom -= 1;
        }

        // Left side top/bottom edges
        let mut x_left = start;
        for crossing in 0..crossings.len() {
            let (inset, l, h) = crossings[crossing];

            // Top edge fill
            for row in 0..inset as usize {
                let idx = row * width as usize + x_left;
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }

            // Top outer edge pixel
            let pixel_idx = inset as usize * width as usize + x_left;
            if PREMULTIPLIED {
                pixels[pixel_idx] = scale_alpha(light_colour, h);
            } else {
                pixels[pixel_idx] = (light_colour & 0x00FFFFFF) | ((h as u32) << 24);
            }
            if h < 255 {
                hit_test_map[pixel_idx] = HIT_NONE;
            }

            // Top inner edge pixel
            let pixel_idx = (inset as usize + 1) * width as usize + x_left;
            pixels[pixel_idx] = blend_rgb_only(pixels[pixel_idx], light_colour, h, l);

            // Bottom outer edge pixel
            let pixel_idx = (height as usize - 1 - inset as usize) * width as usize + x_left;
            if PREMULTIPLIED {
                pixels[pixel_idx] = scale_alpha(shadow_colour, h);
            } else {
                pixels[pixel_idx] = (shadow_colour & 0x00FFFFFF) | ((h as u32) << 24);
            }
            if h < 255 {
                hit_test_map[pixel_idx] = HIT_NONE;
            }

            // Bottom inner edge pixel
            let pixel_idx = (height as usize - 2 - inset as usize) * width as usize + x_left;
            pixels[pixel_idx] = blend_rgb_only(pixels[pixel_idx], shadow_colour, h, l);

            // Bottom edge fill
            for row in (height as usize - inset as usize)..height as usize {
                let idx = row * width as usize + x_left;
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }

            x_left += 1;
        }

        // Right side top/bottom edges
        let mut x_right = width as usize - start - 1;
        for crossing in 0..crossings.len() {
            let (inset, l, h) = crossings[crossing];

            // Top edge fill
            for row in 0..inset as usize {
                let idx = row * width as usize + x_right;
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }

            // Top outer edge pixel
            let pixel_idx = inset as usize * width as usize + x_right;
            if PREMULTIPLIED {
                pixels[pixel_idx] = scale_alpha(light_colour, h);
            } else {
                pixels[pixel_idx] = (light_colour & 0x00FFFFFF) | ((h as u32) << 24);
            }
            if h < 255 {
                hit_test_map[pixel_idx] = HIT_NONE;
            }

            // Top inner edge pixel
            let pixel_idx = (inset as usize + 1) * width as usize + x_right;
            pixels[pixel_idx] = blend_rgb_only(pixels[pixel_idx], light_colour, h, l);

            // Bottom outer edge pixel
            let pixel_idx = (height as usize - 1 - inset as usize) * width as usize + x_right;
            if PREMULTIPLIED {
                pixels[pixel_idx] = scale_alpha(shadow_colour, h);
            } else {
                pixels[pixel_idx] = (shadow_colour & 0x00FFFFFF) | ((h as u32) << 24);
            }
            if h < 255 {
                hit_test_map[pixel_idx] = HIT_NONE;
            }

            // Bottom inner edge pixel
            let pixel_idx = (height as usize - 2 - inset as usize) * width as usize + x_right;
            pixels[pixel_idx] = blend_rgb_only(pixels[pixel_idx], shadow_colour, h, l);

            // Bottom edge fill
            for row in (height as usize - inset as usize)..height as usize {
                let idx = row * width as usize + x_right;
                pixels[idx] = 0;
                hit_test_map[idx] = HIT_NONE;
            }

            x_right -= 1;
        }
    }

    /// Apply hover effect to button using cached pixel list
    pub fn draw_button_hairlines(
        pixels: &mut [u32],
        hit_test_map: &mut [u8],
        window_width: u32,
        _window_height: u32,
        button_x_start: usize,
        button_height: usize,
        _start: usize,
        _crossings: &[(u16, u8, u8)],
    ) {
        let width = window_width as usize;
        let y_start = 0;

        // button_width equals button_height (passed in, already scaled with span * ru)
        let button_width = button_height;

        // Two hairlines: at 1.0 and 2.0 button widths from button area start
        // Left hairline between minimize and maximize
        let left_px = button_x_start + button_width;
        // Right hairline between maximize and close
        let right_px = button_x_start + button_width * 2;

        // Start from vertical center and draw upward until we hit transparency
        let center_y = y_start + button_height / 2;

        // Edge/hairline colour
        let edge_colour = theme::WINDOW_CONTROLS_HAIRLINE;

        // Draw left hairline
        // Draw upward from center until colour changes
        let center_colour = pixels[center_y * width + left_px];
        for py in (y_start..=center_y).rev() {
            let idx = py * width + left_px;
            let diff = pixels[idx] != center_colour;
            pixels[idx] = edge_colour;
            hit_test_map[idx] = HIT_NONE;
            if diff {
                break;
            }
        }

        // Draw downward from center+1 until colour changes
        for py in (center_y + 1)..(y_start + button_height) {
            let idx = py * width + left_px;
            let diff = pixels[idx] != center_colour;
            pixels[idx] = edge_colour;
            hit_test_map[idx] = HIT_NONE;
            if diff {
                break;
            }
        }

        // Draw right hairline
        // Draw upward from center until colour changes
        let center_colour_right = pixels[center_y * width + right_px];
        for py in (y_start..=center_y).rev() {
            let idx = py * width + right_px;
            let diff = pixels[idx] != center_colour_right;
            pixels[idx] = edge_colour;
            hit_test_map[idx] = HIT_NONE;
            if diff {
                break;
            }
        }

        // Draw downward from center+1 until colour changes
        for py in (center_y + 1)..(y_start + button_height) {
            let idx = py * width + right_px;
            let diff = pixels[idx] != center_colour_right;
            pixels[idx] = edge_colour;
            hit_test_map[idx] = HIT_NONE;
            if diff {
                break;
            }
        }
    }

    pub fn draw_textbox(
        pixels: &mut [u32],
        hit_test_map: &mut [u8],
        hit_id: u8,
        textbox_mask: &mut [u8],
        window_width: usize,
        center_x: usize,
        center_y: isize, // Signed to handle negative scroll positions
        box_width: usize,
        box_height: usize,
    ) {
        // Buffer length for bounds checking (scroll can push textbox partially off-screen)
        let buf_len = pixels.len();
        let height = buf_len / window_width;
        let height_signed = height as isize;

        // Convert from center coordinates to top-left (signed for correct off-screen handling)
        let x = center_x.wrapping_sub(box_width / 2);
        let y_signed = center_y - (box_height as isize / 2);
        // Wrapped version for existing code that still uses usize (edge pixels)
        let y = if y_signed >= 0 {
            y_signed as usize
        } else {
            0usize.wrapping_sub((-y_signed) as usize)
        };

        // WHY: Check if textbox overlaps visible region [0, height) using signed math
        // PROOF: top = y_signed, bottom = y_signed + box_height; visible if top < height AND bottom > 0
        // PREVENTS: Drawing when entirely off-screen, while allowing partial visibility
        let box_top = y_signed;
        let box_bottom = y_signed + box_height as isize;
        if box_bottom <= 0 || box_top >= height_signed {
            return; // Entirely off-screen
        }

        let light_colour = theme::TEXTBOX_LIGHT_EDGE;
        let shadow_colour = theme::TEXTBOX_SHADOW_EDGE;
        let fill_colour = theme::TEXTBOX_FILL;
        // Pill-shaped: radius = height/2 gives semicircular ends
        let radius = box_height as f32 / 2.;
        let squirdleyness = 3;

        // Generate crossings from edge (radius/12 o'clock) toward diagonal (1:30)
        let mut crossings: Vec<(u16, u8, u8)> = Vec::new();
        let mut offset = 0f32;

        loop {
            let y_norm = offset / radius;
            let x_norm = (1. - y_norm.powi(squirdleyness)).powf(1. / squirdleyness as f32);
            let x = x_norm * radius;
            let inset = radius - x;

            if inset >= 0. {
                let l = (inset.fract().sqrt() * 256.) as u8;
                let h = ((1. - inset.fract()).sqrt() * 256.) as u8;
                crossings.push((inset as u16, l, h));
            }

            // Stop at 45-degree diagonal (when x < offset)
            if x < offset {
                break;
            }

            offset += 1.;
        }

        // (height already computed above for early-out check)

        // Top-left corner - vertical edge with diagonal fill
        // WHY bounds checks: Scroll can push textbox partially off-screen (negative Y wraps to huge usize,
        // or Y exceeds height). X could also exceed width on narrow windows.
        // PROOF: wrapping_add/wrapping_sub produce wrapped coordinates when textbox is off-screen.
        // PREVENTS: Out-of-bounds pixel buffer access when textbox is partially visible.
        for (i, &(inset, l, h)) in crossings.iter().enumerate() {
            // Stop at diagonal - when inset exceeds i, we've gone past the 45-degree point
            if inset as usize > i {
                break;
            }

            let py = y.wrapping_add(radius as usize).wrapping_sub(i); // Start at horizontal center, go up
            let px = x.wrapping_add(inset as usize);

            // Outer antialiased pixel (bounds check justified above)
            if py < height && px < window_width {
                let idx = py * window_width + px;
                pixels[idx] = blend_rgb_only(pixels[idx], light_colour, l, h);
            }

            // Inner antialiased pixel
            let px1 = px.wrapping_add(1);
            if py < height && px1 < window_width {
                let idx = py * window_width + px1;
                pixels[idx] = blend_rgb_only(light_colour, fill_colour, l, h);
                hit_test_map[idx] = hit_id;
                textbox_mask[idx] = h;
            }

            // Fill horizontally to the diagonal (where horizontal edge would be)
            if py < height {
                let diag_x = x
                    .wrapping_add(radius as usize)
                    .wrapping_sub(i)
                    .min(window_width);
                for fill_x in (px.wrapping_add(2))..=diag_x {
                    if fill_x >= window_width {
                        continue;
                    }
                    let idx = py * window_width + fill_x;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    textbox_mask[idx] = 255;
                }
            }

            // Horizontal edge - Outer antialiased pixel
            let hx = x.wrapping_add(radius as usize).wrapping_sub(i); // Start at vertical center, go left
            let hy = y.wrapping_add(inset as usize); // Distance from top edge

            if hy < height && hx < window_width {
                let idx = hy * window_width + hx;
                pixels[idx] = blend_rgb_only(pixels[idx], light_colour, l, h);
            }

            // Horizontal edge - Inner antialiased pixel (below the outer)
            let hy1 = hy.wrapping_add(1);
            if hy1 < height && hx < window_width {
                let idx = hy1 * window_width + hx;
                pixels[idx] = blend_rgb_only(light_colour, fill_colour, l, h);
                hit_test_map[idx] = hit_id;
                textbox_mask[idx] = h;
            }

            // Fill vertically between horizontal edge and diagonal
            // WHY: Use signed arithmetic to handle negative Y when scrolled off top
            // PROOF: y_signed can be negative, clamping to [0, height) gives visible portion
            // PREVENTS: Infinite loop from wrapped usize, fills only visible pixels
            let hy_signed = y_signed + inset as isize;
            let diag_y_signed = y_signed + radius as isize - i as isize;
            let fill_start = (hy_signed + 2).max(0).min(height_signed) as usize;
            let fill_end = diag_y_signed.max(0).min(height_signed) as usize;
            if hx < window_width && fill_start < fill_end {
                for fill_y in fill_start..fill_end {
                    let idx = fill_y * window_width + hx;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    textbox_mask[idx] = 255;
                }
            }
        }

        // Top-right corner - mirror of top-left (flip x)
        // (bounds checks justified in top-left comment: scroll causes partial visibility)
        for (i, &(inset, l, h)) in crossings.iter().enumerate() {
            if inset as usize > i {
                break;
            }

            let py = y.wrapping_add(radius as usize).wrapping_sub(i);
            let px = x
                .wrapping_add(box_width)
                .wrapping_sub(1)
                .wrapping_sub(inset as usize);

            // Vertical edge - Outer antialiased pixel
            if py < height && px < window_width {
                let idx = py * window_width + px;
                pixels[idx] = blend_rgb_only(pixels[idx], shadow_colour, l, h);
            }

            // Vertical edge - Inner antialiased pixel
            let px1 = px.wrapping_sub(1);
            if py < height && px1 < window_width {
                let idx = py * window_width + px1;
                pixels[idx] = blend_rgb_only(shadow_colour, fill_colour, l, h);
                hit_test_map[idx] = hit_id;
                textbox_mask[idx] = h;
            }

            // Fill horizontally to the diagonal
            if py < height {
                let diag_x = x
                    .wrapping_add(box_width)
                    .wrapping_sub(1)
                    .wrapping_sub(radius as usize)
                    .wrapping_add(i);
                for fill_x in diag_x..px.wrapping_sub(1) {
                    if fill_x >= window_width {
                        continue;
                    }
                    let idx = py * window_width + fill_x;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    textbox_mask[idx] = 255;
                }
            }

            // Horizontal edge - Outer antialiased pixel
            let hx = x
                .wrapping_add(box_width)
                .wrapping_sub(1)
                .wrapping_sub(radius as usize)
                .wrapping_add(i);
            let hy = y.wrapping_add(inset as usize);

            if hy < height && hx < window_width {
                let idx = hy * window_width + hx;
                pixels[idx] = blend_rgb_only(pixels[idx], light_colour, l, h);
            }

            // Horizontal edge - Inner antialiased pixel
            let hy1 = hy.wrapping_add(1);
            if hy1 < height && hx < window_width {
                let idx = hy1 * window_width + hx;
                pixels[idx] = blend_rgb_only(light_colour, fill_colour, l, h);
                hit_test_map[idx] = hit_id;
                textbox_mask[idx] = h;
            }

            // Fill vertically between horizontal edge and diagonal
            // WHY: Use signed arithmetic to handle negative Y when scrolled off top
            // PROOF: y_signed can be negative, clamping to [0, height) gives visible portion
            // PREVENTS: Infinite loop from wrapped usize, fills only visible pixels
            let hy_signed = y_signed + inset as isize;
            let diag_y_signed = y_signed + radius as isize - i as isize;
            let fill_start = (hy_signed + 2).max(0).min(height_signed) as usize;
            let fill_end = diag_y_signed.max(0).min(height_signed) as usize;
            if hx < window_width && fill_start < fill_end {
                for fill_y in fill_start..fill_end {
                    let idx = fill_y * window_width + hx;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    textbox_mask[idx] = 255;
                }
            }
        }

        // Bottom-left corner - mirror of top-left (flip y), shifted down 1 to avoid overlap
        // (bounds checks justified in top-left comment: scroll causes partial visibility)
        for (i, &(inset, l, h)) in crossings.iter().enumerate() {
            if inset as usize > i {
                break;
            }

            let py = y
                .wrapping_add(box_height)
                .wrapping_sub(radius as usize)
                .wrapping_add(i);
            let px = x.wrapping_add(inset as usize);

            // Vertical edge - Outer antialiased pixel
            if py < height && px < window_width {
                let idx = py * window_width + px;
                pixels[idx] = blend_rgb_only(pixels[idx], light_colour, l, h);
            }

            // Vertical edge - Inner antialiased pixel
            let px1 = px.wrapping_add(1);
            if py < height && px1 < window_width {
                let idx = py * window_width + px1;
                pixels[idx] = blend_rgb_only(light_colour, fill_colour, l, h);
                hit_test_map[idx] = hit_id;
                textbox_mask[idx] = h;
            }

            // Fill horizontally to the diagonal
            if py < height {
                let diag_x = x
                    .wrapping_add(radius as usize)
                    .wrapping_sub(i)
                    .min(window_width);
                for fill_x in (px.wrapping_add(2))..=diag_x {
                    if fill_x >= window_width {
                        continue;
                    }
                    let idx = py * window_width + fill_x;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    textbox_mask[idx] = 255;
                }
            }

            // Horizontal edge - Outer antialiased pixel
            let hx = x.wrapping_add(radius as usize).wrapping_sub(i);
            let hy = y.wrapping_add(box_height).wrapping_sub(inset as usize);

            if hy < height && hx < window_width {
                let idx = hy * window_width + hx;
                pixels[idx] = blend_rgb_only(pixels[idx], shadow_colour, l, h);
            }

            // Horizontal edge - Inner antialiased pixel
            let hy1 = hy.wrapping_sub(1);
            if hy1 < height && hx < window_width {
                let idx = hy1 * window_width + hx;
                pixels[idx] = blend_rgb_only(shadow_colour, fill_colour, l, h);
                hit_test_map[idx] = hit_id;
                textbox_mask[idx] = h;
            }

            // Fill vertically between diagonal and horizontal edge
            // WHY: Use signed arithmetic to handle off-screen scroll
            // PROOF: y_signed can be negative or exceed height, clamping gives visible portion
            // PREVENTS: Infinite loop from wrapped usize, fills only visible pixels
            let diag_y_signed = y_signed + box_height as isize - radius as isize + i as isize;
            let hy_signed = y_signed + box_height as isize - inset as isize;
            let fill_start = (diag_y_signed + 1).max(0).min(height_signed) as usize;
            let fill_end = (hy_signed - 1).max(0).min(height_signed) as usize;
            if hx < window_width && fill_start < fill_end {
                for fill_y in fill_start..fill_end {
                    let idx = fill_y * window_width + hx;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    textbox_mask[idx] = 255;
                }
            }
        }

        // Bottom-right corner - mirror of top-left (flip both x and y), shifted down 1 to avoid overlap
        // (bounds checks justified in top-left comment: scroll causes partial visibility)
        for (i, &(inset, l, h)) in crossings.iter().enumerate() {
            if inset as usize > i {
                break;
            }

            let py = y
                .wrapping_add(box_height)
                .wrapping_sub(radius as usize)
                .wrapping_add(i);
            let px = x
                .wrapping_add(box_width)
                .wrapping_sub(1)
                .wrapping_sub(inset as usize);

            // Vertical edge - Outer antialiased pixel
            if py < height && px < window_width {
                let idx = py * window_width + px;
                pixels[idx] = blend_rgb_only(pixels[idx], shadow_colour, l, h);
            }

            // Vertical edge - Inner antialiased pixel
            let px1 = px.wrapping_sub(1);
            if py < height && px1 < window_width {
                let idx = py * window_width + px1;
                pixels[idx] = blend_rgb_only(shadow_colour, fill_colour, l, h);
                hit_test_map[idx] = hit_id;
                textbox_mask[idx] = h;
            }

            // Fill horizontally to the diagonal
            if py < height {
                let diag_x = x
                    .wrapping_add(box_width)
                    .wrapping_sub(1)
                    .wrapping_sub(radius as usize)
                    .wrapping_add(i);
                for fill_x in diag_x..px.wrapping_sub(1) {
                    if fill_x >= window_width {
                        continue;
                    }
                    let idx = py * window_width + fill_x;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    textbox_mask[idx] = 255;
                }
            }

            // Horizontal edge - Outer antialiased pixel
            let hx = x
                .wrapping_add(box_width)
                .wrapping_sub(1)
                .wrapping_sub(radius as usize)
                .wrapping_add(i);
            let hy = y.wrapping_add(box_height).wrapping_sub(inset as usize);

            if hy < height && hx < window_width {
                let idx = hy * window_width + hx;
                pixels[idx] = blend_rgb_only(pixels[idx], shadow_colour, l, h);
            }

            // Horizontal edge - Inner antialiased pixel
            let hy1 = hy.wrapping_sub(1);
            if hy1 < height && hx < window_width {
                let idx = hy1 * window_width + hx;
                pixels[idx] = blend_rgb_only(shadow_colour, fill_colour, l, h);
                hit_test_map[idx] = hit_id;
                textbox_mask[idx] = h;
            }

            // Fill vertically between diagonal and horizontal edge
            // WHY: Use signed arithmetic to handle off-screen scroll
            // PROOF: y_signed can be negative or exceed height, clamping gives visible portion
            // PREVENTS: Infinite loop from wrapped usize, fills only visible pixels
            let diag_y_signed = y_signed + box_height as isize - radius as isize + i as isize;
            let hy_signed = y_signed + box_height as isize - inset as isize;
            let fill_start = (diag_y_signed + 1).max(0).min(height_signed) as usize;
            let fill_end = (hy_signed - 1).max(0).min(height_signed) as usize;
            if hx < window_width && fill_start < fill_end {
                for fill_y in fill_start..fill_end {
                    let idx = fill_y * window_width + hx;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    textbox_mask[idx] = 255;
                }
            }
        }

        // Fill center and straight edges
        // Use signed arithmetic to clamp Y ranges to visible portion
        let radius_int = radius as isize;

        if box_width > box_height {
            // Fat box: draw top and bottom straight edges
            let left_edge = x.wrapping_add(radius as usize);
            let right_edge = x.wrapping_add(box_width).wrapping_sub(radius as usize);

            // Top edge (horizontal hairline) - just outer pixel
            let top_y = y_signed.max(0).min(height_signed) as usize;
            if y_signed >= 0 && y_signed < height_signed {
                for px in left_edge..right_edge {
                    if px >= window_width {
                        continue;
                    }
                    let idx = top_y * window_width + px;
                    pixels[idx] = light_colour;
                }
            }

            // Bottom edge (horizontal hairline) - just outer pixel, shifted down 1
            let bottom_y_signed = y_signed + box_height as isize;
            if bottom_y_signed >= 0 && bottom_y_signed < height_signed {
                let bottom_y = bottom_y_signed as usize;
                for px in left_edge..right_edge {
                    if px >= window_width {
                        continue;
                    }
                    let idx = bottom_y * window_width + px;
                    pixels[idx] = shadow_colour;
                }
            }

            // Fill center rectangle - clamp Y range to visible portion
            let fill_top = (y_signed + 1).max(0).min(height_signed) as usize;
            let fill_bottom = (y_signed + box_height as isize).max(0).min(height_signed) as usize;
            for py in fill_top..fill_bottom {
                for px in left_edge..right_edge {
                    if px >= window_width {
                        continue;
                    }
                    let idx = py * window_width + px;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    textbox_mask[idx] = 255;
                }
            }
        } else {
            // Skinny box: draw left and right straight edges
            let top_edge = (y_signed + radius_int).max(0).min(height_signed) as usize;
            let bottom_edge = (y_signed + box_height as isize - radius_int)
                .max(0)
                .min(height_signed) as usize;

            // Left edge (vertical hairline) - just outer pixel
            if x < window_width {
                for py in top_edge..bottom_edge {
                    let idx = py * window_width + x;
                    pixels[idx] = light_colour;
                }
            }

            // Right edge (vertical hairline) - just outer pixel
            let right_x = x.wrapping_add(box_width);
            if right_x < window_width {
                for py in top_edge..bottom_edge {
                    let idx = py * window_width + right_x;
                    pixels[idx] = shadow_colour;
                }
            }

            // Fill center rectangle
            for py in top_edge..bottom_edge {
                for px in (x.wrapping_add(1))..(x.wrapping_add(box_width).wrapping_sub(1)) {
                    if px >= window_width {
                        continue;
                    }
                    let idx = py * window_width + px;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    textbox_mask[idx] = 255;
                }
            }
        }
    }

    /// Generate textbox glow mask by blurring textbox_mask left/right and knocking out center
    /// glow_colour is 0x00RRGGBB format (no alpha), or 0x00010101 for white/gray
    pub fn apply_textbox_glow(
        pixels: &mut [u32],
        textbox_mask: &[u8],
        window_width: usize,
        center_y: isize,
        box_width: usize,
        box_height: usize,
        add: bool,
        glow_colour: u32,
    ) {
        // Blur radii (how far to blur in each direction)
        let blur_radius_horiz = 32;
        let blur_radius_vert = 16;

        // WHY: center_y can be negative or huge when scrolled off-screen
        // PROOF: y_top/y_bottom computed from center_y - need center on-screen for valid usize
        // PREVENTS: Underflow when computing y_top = center_y - box_height/2
        // NOTE: Cast to usize wraps negatives to huge values, failing >= height check
        let height = pixels.len() / window_width;
        let half_h = (box_height / 2) as isize;
        if (center_y - half_h) as usize >= height || (center_y + half_h) as usize >= height {
            return;
        }
        let center_y = center_y as usize;

        // Textbox bounds (guard above ensures these + blur_radius stay in bounds)
        let y_top = center_y - box_height / 2;
        let y_bottom = center_y + box_height / 2;

        // Find horizontal bounds of textbox (left/right edges)
        let center_x = window_width / 2;
        let mut x_left = center_x;
        let mut x_right = center_x;

        // Scan from center to find textbox edges
        let scan_y = center_y * window_width;
        for x in (0..center_x).rev() {
            if textbox_mask[scan_y + x] > 0 {
                x_left = x;
            } else {
                break;
            }
        }
        for x in center_x..window_width {
            if textbox_mask[scan_y + x] > 0 {
                x_right = x;
            } else {
                break;
            }
        }

        // Corner radius for skipping rounded corners (harmonic mean for smooth scaling)
        let corner_radius = 2 * box_width * box_height / (box_width + box_height);
        let x_vert_start = x_left + corner_radius;
        let x_vert_end = x_right - corner_radius;
        let y_horiz_start = y_top + corner_radius;
        let y_horiz_end = y_bottom - corner_radius;

        let mut adder;
        if add {
            // Horizontal blur pass - right from right edge (skip rounded corners)
            for y in y_top..y_bottom {
                adder = 0;
                for x in x_right
                    - (y_horiz_start as isize - y as isize).max(0) as usize
                    - (y as isize - y_horiz_end as isize).max(0) as usize
                    ..x_right + blur_radius_horiz
                {
                    let idx = y * window_width + x;
                    if x > 0 && textbox_mask[idx] < textbox_mask[idx - 1] {
                        adder += (textbox_mask[idx - 1] - textbox_mask[idx]) as u32;
                    }
                    adder = (adder * 15 >> 4).min(71);
                    let intensity = (adder * (255 - textbox_mask[idx]) as u32) >> 8;
                    let r = ((glow_colour >> 16) & 0xFF) * intensity >> 8;
                    let g = ((glow_colour >> 8) & 0xFF) * intensity >> 8;
                    let b = (glow_colour & 0xFF) * intensity >> 8;
                    pixels[idx] += (r << 16) | (g << 8) | b;
                }
            }

            // Horizontal blur pass - left from left edge (with diagonal corner fill)
            for y in y_top..y_bottom {
                adder = 0;
                // PROOF saturating_sub: blur_radius_horiz could exceed x_left
                // Prevents underflow when blurring near left edge, saturating at 0
                for x in (x_left.saturating_sub(blur_radius_horiz)
                    ..=x_left
                        + (y_horiz_start as isize - y as isize).max(0) as usize
                        + (y as isize - y_horiz_end as isize).max(0) as usize)
                    .rev()
                {
                    let idx = y * window_width + x;
                    if x + 1 < window_width && textbox_mask[idx] < textbox_mask[idx + 1] {
                        adder += (textbox_mask[idx + 1] - textbox_mask[idx]) as u32;
                    }
                    adder = (adder * 15 >> 4).min(71);
                    let intensity = (adder * (255 - textbox_mask[idx]) as u32) >> 8;
                    let r = ((glow_colour >> 16) & 0xFF) * intensity >> 8;
                    let g = ((glow_colour >> 8) & 0xFF) * intensity >> 8;
                    let b = (glow_colour & 0xFF) * intensity >> 8;
                    pixels[idx] += (r << 16) | (g << 8) | b;
                }
            }

            // Vertical blur pass - down from bottom edge (with diagonal corner fill)
            for x in x_left..x_right {
                adder = 0;
                for y in y_bottom
                    - (x_vert_start as isize - x as isize).max(0) as usize
                    - (x as isize - x_vert_end as isize).max(0) as usize
                    ..y_bottom + blur_radius_vert
                {
                    // WHY: Glow extends blur_radius_vert below textbox, may exceed screen
                    // PROOF: Loop scans outward (increasing y), once y >= height all remaining y's also >=
                    // PREVENTS: Out-of-bounds pixel access when glow extends past bottom edge
                    if y >= height {
                        break;
                    }
                    let idx = y * window_width + x;
                    if y > 0 {
                        let idx_above = (y - 1) * window_width + x;
                        if textbox_mask[idx] < textbox_mask[idx_above] {
                            adder += (textbox_mask[idx_above] - textbox_mask[idx]) as u32;
                        }
                    }
                    adder = (adder * 3 >> 2).min(70);
                    let intensity = (adder * (255 - textbox_mask[idx]) as u32) >> 8;
                    let r = ((glow_colour >> 16) & 0xFF) * intensity >> 8;
                    let g = ((glow_colour >> 8) & 0xFF) * intensity >> 8;
                    let b = (glow_colour & 0xFF) * intensity >> 8;
                    pixels[idx] += (r << 16) | (g << 8) | b;
                }
            }

            // Vertical blur pass - up from top edge (with diagonal corner fill)
            for x in x_left..x_right {
                adder = 0;
                for y in (0..=y_top
                    + (x_vert_start as isize - x as isize).max(0) as usize
                    + (x as isize - x_vert_end as isize).max(0) as usize)
                    .rev()
                {
                    // WHY: Glow extends blur_radius_vert above textbox, may go negative (wrapped)
                    // PROOF: Loop scans outward (decreasing y), once past y_top - blur_radius_vert we stop
                    // PREVENTS: Processing pixels above glow region or wrapped negative values
                    if y + blur_radius_vert < y_top {
                        break;
                    }
                    // WHY: y_top + corner_adjust can exceed height when textbox is near bottom
                    // PROOF: Loop starts at y_top + corner_adjust, which depends on textbox position
                    // PREVENTS: Out-of-bounds access to textbox_mask[idx] and pixels[idx]
                    if y >= height {
                        continue;
                    }
                    let idx = y * window_width + x;
                    if y + 1 < height {
                        let idx_below = (y + 1) * window_width + x;
                        if textbox_mask[idx] < textbox_mask[idx_below] {
                            adder += (textbox_mask[idx_below] - textbox_mask[idx]) as u32;
                        }
                    }
                    adder = (adder * 3 >> 2).min(70);
                    let intensity = (adder * (255 - textbox_mask[idx]) as u32) >> 8;
                    let r = ((glow_colour >> 16) & 0xFF) * intensity >> 8;
                    let g = ((glow_colour >> 8) & 0xFF) * intensity >> 8;
                    let b = (glow_colour & 0xFF) * intensity >> 8;
                    pixels[idx] += (r << 16) | (g << 8) | b;
                }
            }
        } else {
            // Horizontal blur pass - right from right edge (skip rounded corners)
            for y in y_top..y_bottom {
                adder = 0;
                for x in x_right
                    - (y_horiz_start as isize - y as isize).max(0) as usize
                    - (y as isize - y_horiz_end as isize).max(0) as usize
                    ..x_right + blur_radius_horiz
                {
                    let idx = y * window_width + x;
                    if x > 0 && textbox_mask[idx] < textbox_mask[idx - 1] {
                        adder += (textbox_mask[idx - 1] - textbox_mask[idx]) as u32;
                    }
                    adder = (adder * 15 >> 4).min(71);
                    let intensity = (adder * (255 - textbox_mask[idx]) as u32) >> 8;
                    let r = ((glow_colour >> 16) & 0xFF) * intensity >> 8;
                    let g = ((glow_colour >> 8) & 0xFF) * intensity >> 8;
                    let b = (glow_colour & 0xFF) * intensity >> 8;
                    pixels[idx] -= (r << 16) | (g << 8) | b;
                }
            }

            // Horizontal blur pass - left from left edge (with diagonal corner fill)
            for y in y_top..y_bottom {
                adder = 0;
                // PROOF saturating_sub: blur_radius_horiz could exceed x_left
                // Prevents underflow when blurring near left edge, saturating at 0
                for x in (x_left.saturating_sub(blur_radius_horiz)
                    ..=x_left
                        + (y_horiz_start as isize - y as isize).max(0) as usize
                        + (y as isize - y_horiz_end as isize).max(0) as usize)
                    .rev()
                {
                    let idx = y * window_width + x;
                    if x + 1 < window_width && textbox_mask[idx] < textbox_mask[idx + 1] {
                        adder += (textbox_mask[idx + 1] - textbox_mask[idx]) as u32;
                    }
                    adder = (adder * 15 >> 4).min(71);
                    let intensity = (adder * (255 - textbox_mask[idx]) as u32) >> 8;
                    let r = ((glow_colour >> 16) & 0xFF) * intensity >> 8;
                    let g = ((glow_colour >> 8) & 0xFF) * intensity >> 8;
                    let b = (glow_colour & 0xFF) * intensity >> 8;
                    pixels[idx] -= (r << 16) | (g << 8) | b;
                }
            }

            // Vertical blur pass - down from bottom edge (with diagonal corner fill)
            for x in x_left..x_right {
                adder = 0;
                for y in y_bottom
                    - (x_vert_start as isize - x as isize).max(0) as usize
                    - (x as isize - x_vert_end as isize).max(0) as usize
                    ..y_bottom + blur_radius_vert
                {
                    // WHY: Glow extends blur_radius_vert below textbox, may exceed screen
                    // PROOF: Loop scans outward (increasing y), once y >= height all remaining y's also >=
                    // PREVENTS: Out-of-bounds pixel access when glow extends past bottom edge
                    if y >= height {
                        break;
                    }
                    let idx = y * window_width + x;
                    if y > 0 {
                        let idx_above = (y - 1) * window_width + x;
                        if textbox_mask[idx] < textbox_mask[idx_above] {
                            adder += (textbox_mask[idx_above] - textbox_mask[idx]) as u32;
                        }
                    }
                    adder = (adder * 3 >> 2).min(70);
                    let intensity = (adder * (255 - textbox_mask[idx]) as u32) >> 8;
                    let r = ((glow_colour >> 16) & 0xFF) * intensity >> 8;
                    let g = ((glow_colour >> 8) & 0xFF) * intensity >> 8;
                    let b = (glow_colour & 0xFF) * intensity >> 8;
                    pixels[idx] -= (r << 16) | (g << 8) | b;
                }
            }

            // Vertical blur pass - up from top edge (with diagonal corner fill)
            for x in x_left..x_right {
                adder = 0;
                for y in (0..=y_top
                    + (x_vert_start as isize - x as isize).max(0) as usize
                    + (x as isize - x_vert_end as isize).max(0) as usize)
                    .rev()
                {
                    // WHY: Glow extends blur_radius_vert above textbox, may go negative (wrapped)
                    // PROOF: Loop scans outward (decreasing y), once past y_top - blur_radius_vert we stop
                    // PREVENTS: Processing pixels above glow region or wrapped negative values
                    if y + blur_radius_vert < y_top {
                        break;
                    }
                    // WHY: y_top + corner_adjust can exceed height when textbox is near bottom
                    // PROOF: Loop starts at y_top + corner_adjust, which depends on textbox position
                    // PREVENTS: Out-of-bounds access to textbox_mask[idx] and pixels[idx]
                    if y >= height {
                        continue;
                    }
                    let idx = y * window_width + x;
                    if y + 1 < height {
                        let idx_below = (y + 1) * window_width + x;
                        if textbox_mask[idx] < textbox_mask[idx_below] {
                            adder += (textbox_mask[idx_below] - textbox_mask[idx]) as u32;
                        }
                    }
                    adder = (adder * 3 >> 2).min(70);
                    let intensity = (adder * (255 - textbox_mask[idx]) as u32) >> 8;
                    let r = ((glow_colour >> 16) & 0xFF) * intensity >> 8;
                    let g = ((glow_colour >> 8) & 0xFF) * intensity >> 8;
                    let b = (glow_colour & 0xFF) * intensity >> 8;
                    pixels[idx] -= (r << 16) | (g << 8) | b;
                }
            }
        }
    }

    pub fn draw_button(
        pixels: &mut [u32],
        hit_test_map: &mut [u8],
        mut textbox_mask: Option<&mut [u8]>,
        window_width: usize,
        _window_height: usize,
        center_x: usize,
        center_y: usize,
        box_width: usize,
        box_height: usize,
        hit_id: u8,
        fill_colour: u32,
        light_colour: u32,
        shadow_colour: u32,
    ) {
        // Convert from center coordinates to top-left
        let x = center_x - box_width / 2;
        let y = center_y - box_height / 2;

        // Pill-shaped: radius = height/2 gives semicircular ends (same as textbox)
        let radius = box_height as f32 / 2.;
        let squirdleyness = 3;

        // Generate crossings from edge (radius/12 o'clock) toward diagonal (1:30)
        let mut crossings: Vec<(u16, u8, u8)> = Vec::new();
        let mut offset = 0f32;

        loop {
            let y_norm = offset / radius;
            let x_norm = (1. - y_norm.powi(squirdleyness)).powf(1. / squirdleyness as f32);
            let x = x_norm * radius;
            let inset = radius - x;

            if inset >= 0. {
                let l = (inset.fract().sqrt() * 256.) as u8;
                let h = ((1. - inset.fract()).sqrt() * 256.) as u8;
                crossings.push((inset as u16, l, h));
            }

            // Stop at 45-degree diagonal (when x < offset)
            if x < offset {
                break;
            }

            offset += 1.;
        }

        // Top-left corner - vertical edge with diagonal fill
        for (i, &(inset, l, h)) in crossings.iter().enumerate() {
            // Stop at diagonal - when inset exceeds i, we've gone past the 45-degree point
            if inset as usize > i {
                break;
            }

            let py = y + radius as usize - i; // Start at horizontal center, go up
            let px = x + inset as usize;

            // Outer antialiased pixel
            let idx = py * window_width + px;
            pixels[idx] = blend_rgb_only(pixels[idx], light_colour, l, h);

            // Inner antialiased pixel
            let idx = idx + 1;
            pixels[idx] = blend_rgb_only(light_colour, fill_colour, l, h);
            hit_test_map[idx] = hit_id;
            if let Some(ref mut mask) = textbox_mask {
                mask[idx] = 255 - h;
            }

            // Fill horizontally to the diagonal (where horizontal edge would be)
            let diag_x = (x + radius as usize - i).min(window_width);
            for fill_x in (px + 2)..=diag_x {
                let idx = py * window_width + fill_x;
                pixels[idx] = fill_colour;
                hit_test_map[idx] = hit_id;
                if let Some(ref mut mask) = textbox_mask {
                    mask[idx] = 0;
                }
            }

            // Horizontal edge - Outer antialiased pixel
            let hx = x + radius as usize - i; // Start at vertical center, go left
            let hy = y + inset as usize; // Distance from top edge

            let idx = hy * window_width + hx;
            pixels[idx] = blend_rgb_only(pixels[idx], light_colour, l, h);

            // Horizontal edge - Inner antialiased pixel (below the outer)
            let idx = (hy + 1) * window_width + hx;
            pixels[idx] = blend_rgb_only(light_colour, fill_colour, l, h);
            hit_test_map[idx] = hit_id;
            if let Some(ref mut mask) = textbox_mask {
                mask[idx] = 255 - h;
            }

            // Fill vertically down from horizontal edge to diagonal
            // Diagonal is where the vertical edge is at this same iteration
            let diag_y = y + radius as usize - i;
            for fill_y in (hy + 2)..diag_y {
                let idx = fill_y * window_width + hx;
                pixels[idx] = fill_colour;
                hit_test_map[idx] = hit_id;
                if let Some(ref mut mask) = textbox_mask {
                    mask[idx] = 0;
                }
            }
        }

        // Top-right corner - mirror of top-left (flip x)
        for (i, &(inset, l, h)) in crossings.iter().enumerate() {
            if inset as usize > i {
                break;
            }

            let py = y + radius as usize - i;
            let px = x + box_width - 1 - inset as usize;

            // Vertical edge - Outer antialiased pixel
            let idx = py * window_width + px;
            pixels[idx] = blend_rgb_only(pixels[idx], shadow_colour, l, h);

            // Vertical edge - Inner antialiased pixel
            let idx = idx - 1;
            pixels[idx] = blend_rgb_only(shadow_colour, fill_colour, l, h);
            hit_test_map[idx] = hit_id;
            if let Some(ref mut mask) = textbox_mask {
                mask[idx] = 255 - h;
            }

            // Fill horizontally to the diagonal
            let diag_x = x + box_width - 1 - radius as usize + i;
            for fill_x in diag_x..(px - 1) {
                let idx = py * window_width + fill_x;
                pixels[idx] = fill_colour;
                hit_test_map[idx] = hit_id;
                if let Some(ref mut mask) = textbox_mask {
                    mask[idx] = 0;
                }
            }

            // Horizontal edge - Outer antialiased pixel
            let hx = x + box_width - 1 - radius as usize + i;
            let hy = y + inset as usize;

            let idx = hy * window_width + hx;
            pixels[idx] = blend_rgb_only(pixels[idx], light_colour, l, h);

            // Horizontal edge - Inner antialiased pixel
            let idx = (hy + 1) * window_width + hx;
            pixels[idx] = blend_rgb_only(light_colour, fill_colour, l, h);
            hit_test_map[idx] = hit_id;
            if let Some(ref mut mask) = textbox_mask {
                mask[idx] = 255 - h;
            }

            // Fill vertically down from horizontal edge to diagonal
            let diag_y = y + radius as usize - i;
            for fill_y in (hy + 2)..diag_y {
                let idx = fill_y * window_width + hx;
                pixels[idx] = fill_colour;
                hit_test_map[idx] = hit_id;
                if let Some(ref mut mask) = textbox_mask {
                    mask[idx] = 0;
                }
            }
        }

        // Bottom-left corner - mirror of top-left (flip y), shifted down 1 to avoid overlap
        for (i, &(inset, l, h)) in crossings.iter().enumerate() {
            if inset as usize > i {
                break;
            }

            let py = y + box_height - radius as usize + i;
            let px = x + inset as usize;

            // Vertical edge - Outer antialiased pixel
            let idx = py * window_width + px;
            pixels[idx] = blend_rgb_only(pixels[idx], light_colour, l, h);

            // Vertical edge - Inner antialiased pixel
            let idx = idx + 1;
            pixels[idx] = blend_rgb_only(light_colour, fill_colour, l, h);
            hit_test_map[idx] = hit_id;
            if let Some(ref mut mask) = textbox_mask {
                mask[idx] = 255 - h;
            }

            // Fill horizontally to the diagonal
            let diag_x = (x + radius as usize - i).min(window_width);
            for fill_x in (px + 2)..=diag_x {
                let idx = py * window_width + fill_x;
                pixels[idx] = fill_colour;
                hit_test_map[idx] = hit_id;
                if let Some(ref mut mask) = textbox_mask {
                    mask[idx] = 0;
                }
            }

            // Horizontal edge - Outer antialiased pixel
            let hx = x + radius as usize - i;
            let hy = y + box_height - inset as usize;

            let idx = hy * window_width + hx;
            pixels[idx] = blend_rgb_only(pixels[idx], shadow_colour, l, h);

            // Horizontal edge - Inner antialiased pixel
            let idx = (hy - 1) * window_width + hx;
            pixels[idx] = blend_rgb_only(shadow_colour, fill_colour, l, h);
            hit_test_map[idx] = hit_id;
            if let Some(ref mut mask) = textbox_mask {
                mask[idx] = 255 - h;
            }

            // Fill vertically up from horizontal edge to diagonal
            let diag_y = y + box_height - radius as usize + i;
            for fill_y in (diag_y + 1)..(hy - 1) {
                let idx = fill_y * window_width + hx;
                pixels[idx] = fill_colour;
                hit_test_map[idx] = hit_id;
                if let Some(ref mut mask) = textbox_mask {
                    mask[idx] = 0;
                }
            }
        }

        // Bottom-right corner - mirror of top-left (flip both x and y), shifted down 1 to avoid overlap
        for (i, &(inset, l, h)) in crossings.iter().enumerate() {
            if inset as usize > i {
                break;
            }

            let py = y + box_height - radius as usize + i;
            let px = x + box_width - 1 - inset as usize;

            // Vertical edge - Outer antialiased pixel
            let idx = py * window_width + px;
            pixels[idx] = blend_rgb_only(pixels[idx], shadow_colour, l, h);

            // Vertical edge - Inner antialiased pixel
            let idx = idx - 1;
            pixels[idx] = blend_rgb_only(shadow_colour, fill_colour, l, h);
            hit_test_map[idx] = hit_id;
            if let Some(ref mut mask) = textbox_mask {
                mask[idx] = 255 - h;
            }

            // Fill horizontally to the diagonal
            let diag_x = x + box_width - 1 - radius as usize + i;
            for fill_x in diag_x..(px - 1) {
                let idx = py * window_width + fill_x;
                pixels[idx] = fill_colour;
                hit_test_map[idx] = hit_id;
                if let Some(ref mut mask) = textbox_mask {
                    mask[idx] = 0;
                }
            }

            // Horizontal edge - Outer antialiased pixel
            let hx = x + box_width - 1 - radius as usize + i;
            let hy = y + box_height - inset as usize;

            let idx = hy * window_width + hx;
            pixels[idx] = blend_rgb_only(pixels[idx], shadow_colour, l, h);

            // Horizontal edge - Inner antialiased pixel
            let idx = (hy - 1) * window_width + hx;
            pixels[idx] = blend_rgb_only(shadow_colour, fill_colour, l, h);
            hit_test_map[idx] = hit_id;
            if let Some(ref mut mask) = textbox_mask {
                mask[idx] = 255 - h;
            }

            // Fill vertically up from horizontal edge to diagonal
            let diag_y = y + box_height - radius as usize + i;
            for fill_y in (diag_y + 1)..(hy - 1) {
                let idx = fill_y * window_width + hx;
                pixels[idx] = fill_colour;
                hit_test_map[idx] = hit_id;
                if let Some(ref mut mask) = textbox_mask {
                    mask[idx] = 0;
                }
            }
        }

        // Fill center and straight edges
        let radius_int = radius as usize;

        if box_width > box_height {
            // Fat box: draw top and bottom straight edges
            let left_edge = x + radius_int;
            let right_edge = x + box_width - radius_int;

            // Top edge (horizontal hairline) - just outer pixel
            for px in left_edge..right_edge {
                let idx = y * window_width + px;
                pixels[idx] = light_colour;
            }

            // Bottom edge (horizontal hairline) - just outer pixel, shifted down 1
            let bottom_y = y + box_height;
            for px in left_edge..right_edge {
                let idx = bottom_y * window_width + px;
                pixels[idx] = shadow_colour;
            }

            // Fill center rectangle
            for py in (y + 1)..(y + box_height) {
                for px in left_edge..right_edge {
                    let idx = py * window_width + px;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    if let Some(ref mut mask) = textbox_mask {
                        mask[idx] = 0;
                    }
                }
            }
        } else {
            // Skinny box: draw left and right straight edges
            let top_edge = y + radius_int;
            let bottom_edge = y + box_height - radius_int;

            // Left edge (vertical hairline) - just outer pixel
            for py in top_edge..bottom_edge {
                let idx = py * window_width + x;
                pixels[idx] = light_colour;
            }

            // Right edge (vertical hairline) - just outer pixel
            let right_x = x + box_width;
            for py in top_edge..bottom_edge {
                let idx = py * window_width + right_x;
                pixels[idx] = shadow_colour;
            }

            // Fill center rectangle
            for py in top_edge..bottom_edge {
                for px in (x + 1)..(x + box_width - 1) {
                    let idx = py * window_width + px;
                    pixels[idx] = fill_colour;
                    hit_test_map[idx] = hit_id;
                    if let Some(ref mut mask) = textbox_mask {
                        mask[idx] = 0;
                    }
                }
            }
        }
    }

    fn draw_black_circle(pixels: &mut [u32], width: usize, cx: usize, cy: usize, radius: usize) {
        let r_outer = radius as isize;
        let r_outer2 = r_outer * r_outer;
        let r_inner = (radius - 1) as isize;
        let r_inner2 = r_inner * r_inner;
        let edge_range = r_outer2 - r_inner2; // Width of the AA edge band

        for dy in -r_outer..=r_outer {
            let y = cy as isize + dy;
            if y < 0 || y >= (pixels.len() / width) as isize {
                continue;
            }
            let dy2 = dy * dy;

            for dx in -r_outer..=r_outer {
                let dist2 = dx * dx + dy2;
                if dist2 > r_outer2 {
                    continue;
                }

                let x = (cx as isize + dx) as usize;

                let idx = y as usize * width + x;
                // Calculate alpha: 255 inside (full darken), 0 at edge (no darken)
                let inv_alpha = if dist2 <= r_inner2 {
                    0
                } else {
                    // Linear gradient from inner edge (0) to outer edge (255)
                    (((dist2 - r_inner2) << 8) / edge_range) as u32
                };

                let mut pixel = pixels[idx] as u64;
                pixel = (pixel | (pixel << 16)) & 0x0000FFFF0000FFFF;
                pixel = (pixel | (pixel << 8)) & 0x00FF00FF00FF00FF;
                pixel *= inv_alpha as u64; // Multiply by inv_alpha, not alpha
                pixel = (pixel >> 8) & 0x00FF00FF00FF00FF;
                pixel = (pixel | (pixel >> 8)) & 0x0000FFFF0000FFFF;
                pixel = pixel | (pixel >> 16);
                pixels[idx] = (pixel as u32) | 0xFF000000;
            }
        }
    }

    /// Add or subtract colour from an anti-aliased circle region
    /// Used for the green overlay on the connectivity indicator
    fn draw_filled_circle(
        pixels: &mut [u32],
        width: usize,
        cx: usize,
        cy: usize,
        radius: usize,
        colour: u32,
        add: bool,
    ) {
        let r_outer = radius as isize;
        let r_outer2 = r_outer * r_outer;
        let r_inner = (radius - 1) as isize;
        let r_inner2 = r_inner * r_inner;
        let edge_range = r_outer2 - r_inner2;

        // Widen the color once
        let mut colour_wide = colour as u64;
        colour_wide = (colour_wide | (colour_wide << 16)) & 0x0000FFFF0000FFFF;
        colour_wide = (colour_wide | (colour_wide << 8)) & 0x00FF00FF00FF00FF;

        for dy in -r_outer..=r_outer {
            let y = cy as isize + dy;
            if y < 0 || y >= (pixels.len() / width) as isize {
                continue;
            }
            let dy2 = dy * dy;

            for dx in -r_outer..=r_outer {
                let dist2 = dx * dx + dy2;
                if dist2 > r_outer2 {
                    continue;
                }
                let x = cx as isize + dx;
                if x < 0 || x >= width as isize {
                    continue;
                }
                let idx = y as usize * width + x as usize;
                // Calculate alpha: 255 inside, 0 at edge
                let alpha = if dist2 <= r_inner2 {
                    255
                } else {
                    (((r_outer2 - dist2) << 8) / edge_range) as u32
                };

                // Scale the color by alpha
                let mut scaled_colour = colour_wide * alpha as u64;
                scaled_colour = (scaled_colour >> 8) & 0x00FF00FF00FF00FF;

                // Narrow back to u32
                scaled_colour = (scaled_colour | (scaled_colour >> 8)) & 0x0000FFFF0000FFFF;
                scaled_colour = scaled_colour | (scaled_colour >> 16);
                let scaled_colour_u32 = scaled_colour as u32;

                // Add or subtract directly on u32
                pixels[idx] = if add {
                    pixels[idx].wrapping_add(scaled_colour_u32)
                } else {
                    pixels[idx].wrapping_sub(scaled_colour_u32)
                };
            }
        }
    }

    /// Add or subtract a single-pixel hairline circle (anti-aliased ring)
    /// Used for the grey outline on offline indicators
    /// Draws at the outer edge of the circle (same edge as draw_indicator_base AA zone)
    fn draw_indicator_hairline(
        pixels: &mut [u32],
        width: usize,
        cx: usize,
        cy: usize,
        radius: usize,
        colour: u32,
        add: bool,
    ) {
        let r_outer = radius as isize;
        let r_outer2 = r_outer * r_outer;
        let r_inner = (radius - 2) as isize;
        let r_inner2 = r_inner * r_inner;
        let edge_range = r_outer2 - r_inner2;

        // Widen the color once
        let mut colour_wide = colour as u64;
        colour_wide = (colour_wide | (colour_wide << 16)) & 0x0000FFFF0000FFFF;
        colour_wide = (colour_wide | (colour_wide << 8)) & 0x00FF00FF00FF00FF;

        for dy in -r_outer..=r_outer {
            let y = cy as isize + dy;
            if y < 0 || y >= (pixels.len() / width) as isize {
                continue;
            }
            let dy2 = dy * dy;

            for dx in -r_outer..=r_outer {
                let dist2 = dx * dx + dy2;
                if dist2 > r_outer2 {
                    continue;
                }
                let x = cx as isize + dx;
                if x < 0 || x >= width as isize {
                    continue;
                }
                let idx = y as usize * width + x as usize;
                // Calculate alpha: 255 inside, 0 at edge
                let alpha = if dist2 <= r_inner2 {
                    continue;
                } else {
                    ((r_outer2 - dist2).min(dist2 - r_inner2) << 9) / edge_range
                };

                // Scale the color by alpha
                let mut scaled_colour = colour_wide * alpha as u64;
                scaled_colour = (scaled_colour >> 8) & 0x00FF00FF00FF00FF;

                // Narrow back to u32
                scaled_colour = (scaled_colour | (scaled_colour >> 8)) & 0x0000FFFF0000FFFF;
                scaled_colour = scaled_colour | (scaled_colour >> 16);
                let scaled_colour_u32 = scaled_colour as u32;

                // Add or subtract directly on u32
                pixels[idx] = if add {
                    pixels[idx].wrapping_add(scaled_colour_u32)
                } else {
                    pixels[idx].wrapping_sub(scaled_colour_u32)
                };
            }
        }
    }

    /// Unified avatar drawing function
    /// - hit_test_map: Some = fill with HIT_AVATAR, None = skip hit testing
    /// - ring_colour: Some = draw status ring, None = no ring
    /// - brighten: brighten avatar when file hovering (self avatar only)
    /// avatar_scaled must be pre-scaled to diameter×diameter (diameter = radius * 2)
    ///
    /// Coordinates are isize to support scrolling (can be partially/fully off-screen).
    /// Computes intersection of avatar bounds with screen - loop bounds prove safety.
    pub fn draw_avatar(
        pixels: &mut [u32],
        mut hit_test_map: Option<&mut [u8]>,
        width: usize,
        height: usize,
        cx: isize,
        cy: isize,
        radius: isize,
        avatar_scaled: Option<&[u8]>,
        ring_colour: Option<u32>,
        brighten: bool,
    ) {
        let r = radius;
        let diameter = (radius * 2) as usize;
        // rhe tweak vs photon: +2 floor so the ring stays visible
        // at tiny (22px tray-icon) radii where radius/16 rounds to 0.
        let stroke_width = radius / 16 + 2;

        // Ring radii: 1px inside + 1px outside = 2px total
        let r_inner = r - 1;
        let r_inner2 = r_inner * r_inner;
        let r_inner_inner = r - 2;
        let r_inner_inner2 = r_inner_inner * r_inner_inner;
        let r_outer = r + stroke_width;
        let r_outer2 = r_outer * r_outer;
        let r_outer_outer = r_outer + 1;
        let r_outer_outer2 = r_outer_outer * r_outer_outer;

        // AA diff for outer ring edge: maps [r_outer2, r_outer_outer2) to [255, 0]
        let diff_outer = r_outer_outer2 - r_outer2;
        // AA diff for inner edge (no-ring case): maps [r_inner_inner2, r_inner2) to [255, 0]
        let diff_inner = r_inner2 - r_inner_inner2;

        // Intersection bounds:
        // WHY: cx/cy can be negative or exceed screen bounds due to scroll offset
        // PROOF: We compute intersection of circle bounding box with screen (0..width, 0..height)
        //        If intersection is empty (y_max <= y_min), avatar is off-screen, return early
        //        Otherwise cast to usize is safe since values are in [0, width/height]
        // PREVENTS: Negative isize cast to usize would wrap to huge value causing infinite loop

        if let Some(ring) = ring_colour {
            // === WITH RING ===
            // Compute intersection of outer ring bounds with screen (keep as isize)
            let y_min_i = (cy - r_outer_outer).max(0);
            let y_max_i = (cy + r_outer_outer + 1).min(height as isize);
            let x_min_i = (cx - r_outer_outer).max(0);
            let x_max_i = (cx + r_outer_outer + 1).min(width as isize);

            // Empty intersection = entirely off-screen
            if y_max_i <= y_min_i || x_max_i <= x_min_i {
                return;
            }

            // Safe to cast - values are in [0, width/height]
            let y_min = y_min_i as usize;
            let y_max = y_max_i as usize;
            let x_min = x_min_i as usize;
            let x_max = x_max_i as usize;

            for y in y_min..y_max {
                let dy = y as isize - cy;
                let dy2 = dy * dy;

                for x in x_min..x_max {
                    let dx = x as isize - cx;
                    let dist2 = dx * dx + dy2;
                    let idx = y * width + x;

                    // Hit test covers ring area (not the AA fringe)
                    if let Some(htm) = hit_test_map.as_mut() {
                        if dist2 <= r_outer2 {
                            htm[idx] = HIT_AVATAR;
                        }
                    }

                    if dist2 <= r_inner_inner2 {
                        // Inside inner AA edge - avatar only
                        let colour = sample_avatar(avatar_scaled, dx, dy, r, diameter, brighten);
                        pixels[idx] = 0xFF000000 | colour;
                    } else if dist2 < r_inner2 {
                        // Inner AA edge - blend ring over avatar
                        let colour = sample_avatar(avatar_scaled, dx, dy, r, diameter, brighten);
                        // PROOF: dist2 ∈ (r_inner_inner2, r_inner2), so numerator ∈ (0, diff_inner<<8)
                        // Division maps to (0, 256), cast to u8 is safe (max 255)
                        let alpha = (((dist2 - r_inner_inner2) << 8) / diff_inner) as u8;
                        pixels[idx] = blend_rgb_only(0xFF000000 | colour, ring, 255 - alpha, alpha);
                    } else if dist2 <= r_outer2 {
                        // Solid ring (r_inner to r_outer)
                        pixels[idx] = 0xFF000000 | ring;
                    } else if dist2 <= r_outer_outer2 {
                        // Outer AA edge (r_outer to r_outer_outer) - blend ring to background
                        // PROOF: dist2 ∈ (r_outer2, r_outer_outer2], so numerator ∈ [0, diff_outer<<8)
                        // Division maps to [0, 256), cast to u8 is safe (max 255)
                        let alpha = (((r_outer_outer2 - dist2) << 8) / diff_outer) as u8;
                        pixels[idx] = blend_rgb_only(pixels[idx], ring, 255 - alpha, alpha);
                    }
                }
            }
        } else {
            // === NO RING ===
            // Compute intersection of inner circle bounds with screen (keep as isize)
            let y_min_i = (cy - r_inner).max(0);
            let y_max_i = (cy + r_inner + 1).min(height as isize);
            let x_min_i = (cx - r_inner).max(0);
            let x_max_i = (cx + r_inner + 1).min(width as isize);

            // Empty intersection = entirely off-screen
            if y_max_i <= y_min_i || x_max_i <= x_min_i {
                return;
            }

            // Safe to cast - values are in [0, width/height]
            let y_min = y_min_i as usize;
            let y_max = y_max_i as usize;
            let x_min = x_min_i as usize;
            let x_max = x_max_i as usize;

            for y in y_min..y_max {
                let dy = y as isize - cy;
                let dy2 = dy * dy;

                for x in x_min..x_max {
                    let dx = x as isize - cx;
                    let idx = y * width + x;
                    let dist2 = dx * dx + dy2;

                    // Circle check - loop is square bounding box, clip to circle
                    if dist2 > r_inner2 {
                        continue;
                    }

                    // Hit test (trimmed radius) - already inside circle
                    if let Some(htm) = hit_test_map.as_mut() {
                        htm[idx] = HIT_AVATAR;
                    }

                    if dist2 <= r_inner_inner2 {
                        let colour = sample_avatar(avatar_scaled, dx, dy, r, diameter, brighten);
                        pixels[idx] = 0xFF000000 | colour;
                    } else {
                        // AA edge - blend avatar to background
                        let colour = sample_avatar(avatar_scaled, dx, dy, r, diameter, brighten);
                        // PROOF: dist2 ∈ (r_inner_inner2, r_inner2], so numerator ∈ [0, diff_inner<<8)
                        // Division maps to [0, 256), cast to u8 is safe (max 255)
                        let alpha = (((r_inner2 - dist2) << 8) / diff_inner) as u8;
                        pixels[idx] = blend_rgb_only(pixels[idx], colour, 255 - alpha, alpha);
                    }
                }
            }
        }
    }
}

/// Sample avatar texture at offset (dx, dy) from center
/// Texture is diameter×diameter, centered at (r, r)
#[inline]
fn sample_avatar(
    avatar_data: Option<&[u8]>,
    dx: isize,
    dy: isize,
    r: isize,
    diameter: usize,
    brighten: bool,
) -> u32 {
    if let Some(data) = avatar_data {
        let tex_x = (dx + r) as usize;
        let tex_y = (dy + r) as usize;
        let tex_idx = (tex_y * diameter + tex_x) * 3;
        let mut red = data[tex_idx] as u32;
        let mut green = data[tex_idx + 1] as u32;
        let mut blue = data[tex_idx + 2] as u32;
        if brighten {
            // PROOF: red/green/blue are u8 (0-255), multiplied by 3/2 gives max 382.
            // .min(255) is REQUIRED to prevent overflow when packing to u32 RGB.
            red = (red * 3 / 2).min(255);
            green = (green * 3 / 2).min(255);
            blue = (blue * 3 / 2).min(255);
        }
        (red << 16) | (green << 8) | blue
    } else {
        if brighten {
            0x404040
        } else {
            0x202020
        }
    }
}

// Helper functions for u32 packed pixel manipulation
// Desktop: ARGB format (0xAARRGGBB)
// Android: ABGR format (0xAABBGGRR)
#[inline]
#[cfg(not(target_os = "android"))]
fn pack_argb(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

#[inline]
#[cfg(target_os = "android")]
fn pack_argb(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((a as u32) << 24) | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32)
}

#[inline]
#[cfg(not(target_os = "android"))]
fn unpack_argb(pixel: u32) -> (u8, u8, u8, u8) {
    let a = (pixel >> 24) as u8;
    let r = (pixel >> 16) as u8;
    let g = (pixel >> 8) as u8;
    let b = pixel as u8;
    (r, g, b, a)
}

#[inline]
#[cfg(target_os = "android")]
fn unpack_argb(pixel: u32) -> (u8, u8, u8, u8) {
    let a = (pixel >> 24) as u8;
    let b = (pixel >> 16) as u8;
    let g = (pixel >> 8) as u8;
    let r = pixel as u8;
    (r, g, b, a)
}

fn scale_alpha(colour: u32, alpha: u8) -> u32 {
    let mut c = colour as u64;
    c = (c | (c << 16)) & 0x0000FFFF0000FFFF;
    c = (c | (c << 8)) & 0x00FF00FF00FF00FF;
    let mut scaled = c * alpha as u64;
    scaled = (scaled >> 8) & 0x00FF00FF00FF00FF;
    scaled = (scaled | (scaled >> 8)) & 0x0000FFFF0000FFFF;
    scaled = scaled | (scaled >> 16);
    scaled as u32
}

#[inline]
fn blend_rgb_only(bg_colour: u32, fg_colour: u32, weight_bg: u8, weight_fg: u8) -> u32 {
    let mut bg = bg_colour as u64;
    bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
    bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

    let mut fg = fg_colour as u64;
    fg = (fg | (fg << 16)) & 0x0000FFFF0000FFFF;
    fg = (fg | (fg << 8)) & 0x00FF00FF00FF00FF;

    // Blend all 4 channels (including alpha)
    let mut blended = bg * weight_bg as u64 + fg * weight_fg as u64;
    blended = (blended >> 8) & 0x00FF00FF00FF00FF;
    blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
    blended = blended | (blended >> 16) | 0xFF000000;

    blended as u32
}
