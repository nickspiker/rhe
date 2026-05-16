use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style, SwashCache, Weight};

pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl TextRenderer {
    pub fn new() -> Self {
        let mut font_system = FontSystem::new();

        let db = font_system.db_mut();

        // Bona Nova — the only face the tutor uses. Regular + italic
        // only; weight is held at 400 throughout. Italic is the per-
        // element style switch (sentence + phoneme line italic;
        // target word / hint / labels regular).
        db.load_font_data(
            include_bytes!("../../../assets/Bona_Nova/static/BonaNova-Regular.ttf").to_vec(),
        );
        db.load_font_data(
            include_bytes!("../../../assets/Bona_Nova/static/BonaNova-Italic.ttf").to_vec(),
        );

        Self {
            font_system,
            swash_cache: SwashCache::new(),
        }
    }

    pub fn draw_text_center_u32(
        &mut self,
        pixels: &mut [u32],
        width: usize,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        weight: u16,
        colour: u32, // [RGBA]
        font: &str,
        italic: bool,
    ) -> f32 {
        let mut attrs = Attrs::new()
            .family(Family::Name(font))
            .weight(Weight(weight));
        if italic {
            attrs = attrs.style(Style::Italic);
        }

        let metrics = Metrics::relative(size, 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        buffer.set_size(&mut self.font_system, None, None);
        buffer.set_text(&mut self.font_system, text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        if let Some(run) = buffer.layout_runs().next() {
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;

            for glyph in run.glyphs {
                min_x = min_x.min(glyph.x);
                max_x = max_x.max(glyph.x + glyph.w);
            }

            let text_width = max_x - min_x;
            let text_height = run.line_height;

            self.render_buffer_u32(
                &mut buffer,
                pixels,
                width,
                x,
                y,
                text_width,
                text_height,
                colour,
                0, // center alignment
            );

            text_width
        } else {
            0.
        }
    }

    pub fn draw_text_left_u32(
        &mut self,
        pixels: &mut [u32],
        width: usize,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        weight: u16,
        colour: u32,
        font: &str,
        italic: bool,
    ) -> f32 {
        let mut attrs = Attrs::new()
            .family(Family::Name(font))
            .weight(Weight(weight));
        if italic {
            attrs = attrs.style(Style::Italic);
        }

        let metrics = Metrics::relative(size, 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        buffer.set_size(&mut self.font_system, None, None);
        buffer.set_text(&mut self.font_system, text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        if let Some(run) = buffer.layout_runs().next() {
            let mut text_width = 0.0f32;
            for glyph in run.glyphs {
                text_width = text_width.max(glyph.x + glyph.w);
            }
            let text_height = run.line_height;

            self.render_buffer_u32(
                &mut buffer,
                pixels,
                width,
                x,
                y,
                text_width,
                text_height,
                colour,
                1, // left alignment
            );

            text_width
        } else {
            0.
        }
    }

    pub fn draw_text_right_u32(
        &mut self,
        pixels: &mut [u32],
        width: usize,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        weight: u16,
        colour: u32,
        font: &str,
        italic: bool,
    ) -> f32 {
        let mut attrs = Attrs::new()
            .family(Family::Name(font))
            .weight(Weight(weight));
        if italic {
            attrs = attrs.style(Style::Italic);
        }

        let metrics = Metrics::relative(size, 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        buffer.set_size(&mut self.font_system, None, None);
        buffer.set_text(&mut self.font_system, text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        if let Some(run) = buffer.layout_runs().next() {
            let mut text_width = 0.0f32;
            for glyph in run.glyphs {
                text_width = text_width.max(glyph.x + glyph.w);
            }
            let text_height = run.line_height;

            self.render_buffer_u32(
                &mut buffer,
                pixels,
                width,
                x,
                y,
                text_width,
                text_height,
                colour,
                2, // right alignment
            );

            text_width
        } else {
            0.
        }
    }

    pub fn draw_text_center(
        &mut self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        weight: u16,
        colour: Vec<u8>,
        rotation: u16,
        font: &str, // "Josefin Slab" for UI, "Open Sans" for user content
    ) -> f32 {
        let attrs = Attrs::new()
            .family(Family::Name(font))
            .weight(Weight(weight));

        let metrics = Metrics::relative(size, 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        buffer.set_size(&mut self.font_system, None, None);
        buffer.set_text(&mut self.font_system, text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        if let Some(run) = buffer.layout_runs().next() {
            // Calculate text width
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;

            for glyph in run.glyphs {
                min_x = min_x.min(glyph.x);
                max_x = max_x.max(glyph.x + glyph.w);
            }

            let text_width = max_x - min_x;
            let text_height = run.line_height;

            self.render_buffer(
                &mut buffer,
                pixels,
                width,
                height,
                x,
                y,
                text_width,
                text_height,
                colour,
                rotation,
                0, // center alignment
            );

            text_width
        } else {
            0.0
        }
    }



    /// Render buffer to u32 packed pixel array
    fn render_buffer_u32(
        &mut self,
        buffer: &mut Buffer,
        pixels: &mut [u32], // [ARGB]
        width: usize,
        anchor_x: f32,
        anchor_y: f32,
        text_width: f32,
        text_height: f32,
        colour: u32,   // [ARGB]
        alignment: u8, // 0=center, 1=left, 2=right
    ) {
        // Calculate offset based on alignment
        let (offset_x, offset_y) = match alignment {
            0 => (anchor_x - text_width / 2., anchor_y - text_height / 2.), // center
            1 => (anchor_x, anchor_y - text_height / 2.),                   // left
            2 => (anchor_x - text_width, anchor_y - text_height / 2.),      // right
            _ => (anchor_x, anchor_y),
        };

        let mut colour = colour as u64;
        colour = (colour | (colour << 16)) & 0x0000FFFF0000FFFF;
        colour = (colour | (colour << 8)) & 0x00FF00FF00FF00FF;

        for run in buffer.layout_runs() {
            let baseline_offset = run.line_y;

            for glyph in run.glyphs {
                let physical_glyph = glyph.physical((offset_x, offset_y), 1.);

                if let Some(image) = self
                    .swash_cache
                    .get_image(&mut self.font_system, physical_glyph.cache_key)
                {
                    let glyph_x = physical_glyph.x + image.placement.left;
                    let glyph_y = physical_glyph.y + baseline_offset as i32 - image.placement.top;

                    let glyph_width = image.placement.width as usize;
                    let glyph_height = image.placement.height as usize;

                    for cy in 0..glyph_height {
                        for cx in 0..glyph_width {
                            let alpha = image.data[cy * glyph_width + cx];
                            if alpha > 0 {
                                let final_x = glyph_x as isize + cx as isize;
                                let final_y = glyph_y as isize + cy as isize;

                                // Bounds check to prevent underflow/overflow
                                if final_x < 0 || final_y < 0 || final_x >= width as isize {
                                    continue;
                                }
                                let idx = final_y as usize * width + final_x as usize;
                                if idx >= pixels.len() {
                                    continue;
                                }

                                let mut bg = pixels[idx] as u64;
                                let alpha = alpha as u64;
                                let inv_alpha = (255 - alpha) as u64;

                                bg = (bg | (bg << 16)) & 0x0000FFFF0000FFFF;
                                bg = (bg | (bg << 8)) & 0x00FF00FF00FF00FF;

                                let mut blended = bg * inv_alpha + colour * alpha;

                                blended = (blended >> 8) & 0x00FF00FF00FF00FF;
                                blended = (blended | (blended >> 8)) & 0x0000FFFF0000FFFF;
                                blended = blended | (blended >> 16);
                                pixels[idx] = blended as u32;
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_buffer(
        &mut self,
        buffer: &mut Buffer,
        pixels: &mut [u8],
        width: u32,
        _height: u32,
        anchor_x: f32,
        anchor_y: f32,
        text_width: f32,
        text_height: f32,
        colour: Vec<u8>,
        rotation: u16,
        alignment: u8, // 0=center, 1=left, 2=right
    ) {
        let channels = colour.len();

        // Calculate the offset based on alignment
        let (offset_x, offset_y) = match alignment {
            0 => (anchor_x - text_width / 2.0, anchor_y - text_height / 2.0), // center
            1 => (anchor_x, anchor_y - text_height / 2.0),                    // left
            2 => (anchor_x - text_width, anchor_y - text_height / 2.0),       // right
            _ => (anchor_x, anchor_y),
        };

        for run in buffer.layout_runs() {
            let baseline_offset = run.line_y;

            for glyph in run.glyphs {
                let physical_glyph = glyph.physical((offset_x, offset_y), 1.);

                if let Some(image) = self
                    .swash_cache
                    .get_image(&mut self.font_system, physical_glyph.cache_key)
                {
                    let glyph_x = physical_glyph.x + image.placement.left;
                    let glyph_y = physical_glyph.y + baseline_offset as i32 - image.placement.top;

                    // Draw the glyph bitmap
                    let glyph_width = image.placement.width as usize;
                    let glyph_height = image.placement.height as usize;

                    for cy in 0..glyph_height {
                        for cx in 0..glyph_width {
                            let alpha = image.data[cy * glyph_width + cx];
                            if alpha > 0 {
                                let py_base = glyph_y + cy as i32;
                                let px_base = glyph_x + cx as i32;

                                // Calculate position relative to anchor point
                                let rel_x = px_base as f32 - anchor_x;
                                let rel_y = py_base as f32 - anchor_y;

                                // Rotate around the anchor point
                                let (rot_x, rot_y) = match rotation {
                                    90 => (rel_y, -rel_x),
                                    180 => (-rel_x, -rel_y),
                                    270 => (-rel_y, rel_x),
                                    _ => (rel_x, rel_y),
                                };

                                // Convert back to absolute coordinates
                                let final_x = (anchor_x + rot_x) as i32;
                                let final_y = (anchor_y + rot_y) as i32;
                                let offset = (final_y as usize * width as usize + final_x as usize)
                                    * channels;

                                let alpha_u16 = alpha as u16;
                                let inv_alpha = 256 - alpha_u16;

                                for c in 0..channels {
                                    pixels[offset + c] = ((pixels[offset + c] as u16 * inv_alpha
                                        + colour[c] as u16 * alpha_u16)
                                        >> 8)
                                        as u8;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

}
