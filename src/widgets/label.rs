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

        // Pre-multiply the text colour and scale to 0–255 range once.
        // All per-pixel blending uses u16 integer math — no floats in the
        // hot path.
        let src_premul = self.text_color.premultiply();
        let src_r = (src_premul.red() * 255.0) as u16;
        let src_g = (src_premul.green() * 255.0) as u16;
        let src_b = (src_premul.blue() * 255.0) as u16;
        let src_a = (src_premul.alpha() * 255.0) as u16;

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

                        // Porter-Duff "over" with integer math.
                        // coverage ∈ [0, 255], src_a ∈ [0, 255]
                        let cov = (coverage * 255.0) as u16;
                        let eff_a = (src_a as u32 * cov as u32 / 255) as u16;
                        let inv_a = 255 - eff_a;

                        let dst_r = dst.red() as u32;
                        let dst_g = dst.green() as u32;
                        let dst_b = dst.blue() as u32;
                        let dst_a = dst.alpha() as u32;

                        let or = ((src_r as u32 * cov as u32 + dst_r * inv_a as u32) / 255) as u8;
                        let og = ((src_g as u32 * cov as u32 + dst_g * inv_a as u32) / 255) as u8;
                        let ob = ((src_b as u32 * cov as u32 + dst_b * inv_a as u32) / 255) as u8;
                        let oa = ((eff_a as u32 + dst_a * inv_a as u32) / 255) as u8;

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
