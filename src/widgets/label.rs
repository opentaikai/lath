use ab_glyph::{Font, ScaleFont};
use tiny_skia::Color;

use crate::core::WidgetId;
use crate::fonts;
use crate::layout::{Constraints, Point, Rect, Size};
use crate::widget::{Widget, WidgetMeasure};

/// A static, single-line text renderer.
///
/// # Layout
///
/// * **measure** – approximates the text bounding box:
///   `width = char_count × font_size × 0.6`, `height = font_size`.
/// * **arrange** – leaf node, returns nothing.
/// * **draw** – rasterises each glyph with `ab_glyph` and paints
///   the resulting coverage mask onto the `tiny-skia` canvas.
pub struct Label {
    text: String,
    text_color: Color,
    font_size: f32,
}

impl Label {
    /// Creates a new label with the given text string.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            text_color: Color::BLACK,
            font_size: 16.0,
        }
    }

    /// Sets the text colour.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Sets the font size in logical pixels.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Returns the current text content.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl<M> Widget<M> for Label {
    fn name(&self) -> &'static str {
        "Label"
    }

    fn children(&self) -> Vec<WidgetId> {
        Vec::new()
    }

    // -- Layout -------------------------------------------------------------

    fn measure(&self, constraints: Constraints, _arena: &dyn WidgetMeasure<M>) -> Size {
        // Approximate monospace glyph width: ~60% of font_size.
        let char_width = self.font_size * 0.6;
        let width = self.text.chars().count() as f32 * char_width;
        let height = self.font_size;

        constraints.constrain(Size { width, height })
    }

    fn arrange(&self, _size: Size, _arena: &dyn WidgetMeasure<M>) -> Vec<(WidgetId, Point)> {
        Vec::new()
    }

    // -- Drawing ------------------------------------------------------------

    fn draw(&self, canvas: &mut tiny_skia::PixmapMut, rect: Rect) {
        let font = fonts::default_font();
        let scale = fonts::scale(self.font_size);
        let scaled_font = font.as_scaled(scale);

        // Pre-multiply the text colour for alpha blending.
        // All values are in 0.0..=1.0 range.
        let src_premul = self.text_color.premultiply();
        let sr = src_premul.red();
        let sg = src_premul.green();
        let sb = src_premul.blue();
        let sa = src_premul.alpha();

        let canvas_w = canvas.width();
        let canvas_h = canvas.height();
        let pixels = canvas.pixels_mut();

        // Starting x position (left edge of the rect).
        let mut cursor_x = rect.origin.x;

        for code_point in self.text.chars() {
            let glyph_id = font.glyph_id(code_point);

            // Advance cursor by the glyph's advance width.
            let advance = scaled_font.h_advance(glyph_id);
            cursor_x += advance;

            // Create a Glyph with scale for outline lookup.
            let glyph = glyph_id.with_scale(self.font_size);

            // Rasterise the glyph and paint coverage samples.
            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();

                // Glyph origin in canvas coordinates.
                let gx = cursor_x + bounds.min.x;
                let gy = rect.origin.y + (scaled_font.ascent() + bounds.min.y);

                outlined.draw(|px, py, coverage| {
                    let x = gx + px as f32;
                    let y = gy + py as f32;

                    if x >= 0.0
                        && y >= 0.0
                        && (x as u32) < canvas_w
                        && (y as u32) < canvas_h
                    {
                        let idx = (y as u32 * canvas_w + x as u32) as usize;
                        let dst = &mut pixels[idx];

                        // Porter-Duff "over" compositing.
                        // The .min(1.0) cap is critical: it ensures out_r <= out_a
                        // (the premultiplied invariant), otherwise from_rgba()
                        // returns None and the glyph pixel is silently dropped.
                        let src_a = sa * coverage;
                        let inv_src_a = 1.0 - src_a;

                        let out_r = (sr * coverage
                            + dst.red() as f32 / 255.0 * inv_src_a)
                            .min(1.0);
                        let out_g = (sg * coverage
                            + dst.green() as f32 / 255.0 * inv_src_a)
                            .min(1.0);
                        let out_b = (sb * coverage
                            + dst.blue() as f32 / 255.0 * inv_src_a)
                            .min(1.0);
                        let out_a = (src_a
                            + dst.alpha() as f32 / 255.0 * inv_src_a)
                            .min(1.0);

                        let or = (out_r * 255.0) as u8;
                        let og = (out_g * 255.0) as u8;
                        let ob = (out_b * 255.0) as u8;
                        let oa = (out_a * 255.0) as u8;

                        *dst = tiny_skia::PremultipliedColorU8::from_rgba(or, og, ob, oa)
                            .unwrap_or(*dst);
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::UiArena;
    use crate::layout::compute_layout;

    #[test]
    fn label_measure_matches_approximate_text_size() {
        let mut arena = UiArena::<String>::new();
        // "Hello" = 5 chars → 5 × 16 × 0.6 = 48.0 width, 16.0 height
        let id = arena.spawn(Label::new("Hello"));
        arena.set_root(id);

        let state = compute_layout(&arena, id, Size { width: 800.0, height: 600.0 });
        let rect = state.get(id).expect("label frame");

        assert!((rect.size.width - 48.0).abs() < f32::EPSILON);
        assert!((rect.size.height - 16.0).abs() < f32::EPSILON);
    }

    #[test]
    fn label_custom_font_size() {
        let mut arena = UiArena::<String>::new();
        // "AB" = 2 chars, font_size 24 → 2 × 24 × 0.6 = 28.8 width, 24 height
        let id = arena.spawn(Label::new("AB").font_size(24.0));
        arena.set_root(id);

        let state = compute_layout(&arena, id, Size { width: 800.0, height: 600.0 });
        let rect = state.get(id).expect("label frame");

        assert!((rect.size.width - 28.8).abs() < 0.01);
        assert!((rect.size.height - 24.0).abs() < 0.01);
    }

    #[test]
    fn label_respects_constraints() {
        let mut arena = UiArena::<String>::new();
        let id = arena.spawn(Label::new("VeryLongText"));
        arena.set_root(id);

        let state = compute_layout(&arena, id, Size { width: 50.0, height: 50.0 });
        let rect = state.get(id).expect("label frame");

        // Unconstrained width = 12 × 16 × 0.6 = 115.2, clamped to 50.0.
        assert!((rect.size.width - 50.0).abs() < f32::EPSILON);
        // Height = 16.0, which is < 50.0 so not clamped.
        assert!((rect.size.height - 16.0).abs() < f32::EPSILON);
    }
}
