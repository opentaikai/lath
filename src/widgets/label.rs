use ab_glyph::{Font, ScaleFont};
use tiny_skia::Color;

use crate::core::WidgetId;
use crate::fonts;
use crate::layout::{Constraints, Point, Rect, Size};
use crate::widget::{Widget, WidgetMeasure};

/// A single-line text label with exact font-metric measurement
/// and glyph-by-glyph rasterization.
///
/// # Layout
///
/// * **measure** – uses `ab_glyph` to sum per-glyph horizontal advance
///   widths for exact text width; height is the font's full line height
///   (`ascent - descent + line_gap`).
/// * **arrange** – leaf node, returns nothing.
/// * **draw** – rasterises each glyph via `ab_glyph::outline_glyph` and
///   blends coverage samples onto the `tiny-skia` canvas.
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

// ---------------------------------------------------------------------------
// Widget impl
// ---------------------------------------------------------------------------

impl<M> Widget<M> for Label {
    fn name(&self) -> &'static str {
        "Label"
    }

    fn children(&self) -> Vec<WidgetId> {
        Vec::new()
    }

    // -- Layout -------------------------------------------------------------

    fn measure(&self, constraints: Constraints, _arena: &dyn WidgetMeasure<M>) -> Size {
        let font = fonts::default_font();
        let scale = fonts::scale(self.font_size);
        let scaled = font.as_scaled(scale);

        let width: f32 = self
            .text
            .chars()
            .map(|c| scaled.h_advance(font.glyph_id(c)))
            .sum();
        let height = scaled.height();

        constraints.constrain(Size { width, height })
    }

    fn arrange(&self, _size: Size, _arena: &dyn WidgetMeasure<M>) -> Vec<(WidgetId, Point)> {
        Vec::new()
    }

    // -- Drawing ------------------------------------------------------------

    fn draw(&self, canvas: &mut tiny_skia::PixmapMut, rect: Rect) {
        let font = fonts::default_font();
        let scale = fonts::scale(self.font_size);
        let scaled = font.as_scaled(scale);

        // Pre-multiplied source colour (used inside the coverage loop).
        let src_premul = self.text_color.premultiply();
        let sr = src_premul.red();
        let sg = src_premul.green();
        let sb = src_premul.blue();
        let sa = src_premul.alpha();

        let canvas_w = canvas.width();
        let canvas_h = canvas.height();
        let pixels = canvas.pixels_mut();

        // Baseline: the layout rect's top-edge + ascent (ascent is positive).
        let baseline_y = rect.origin.y + scaled.ascent();
        let mut cursor_x = rect.origin.x;

        for code_point in self.text.chars() {
            let glyph_id = font.glyph_id(code_point);
            let advance = scaled.h_advance(glyph_id);

            // Position the glyph at the current cursor on the baseline,
            // then advance the cursor for the next glyph.
            let origin_x = cursor_x;
            cursor_x += advance;

            let glyph =
                glyph_id.with_scale_and_position(scale, ab_glyph::point(origin_x, baseline_y));

            if let Some(outlined) = font.outline_glyph(glyph) {
                outlined.draw(|px, py, coverage| {
                    let x = origin_x + px as f32;
                    let y = baseline_y + py as f32;

                    if x >= 0.0 && y >= 0.0 && (x as u32) < canvas_w && (y as u32) < canvas_h {
                        let idx = (y as u32 * canvas_w + x as u32) as usize;
                        let dst = &mut pixels[idx];

                        // Porter-Duff "over" compositing with .min(1.0) to
                        // preserve the premultiplied invariant (r <= a).
                        let src_a = sa * coverage;
                        let inv_src_a = 1.0 - src_a;

                        let out_r = (sr * coverage + dst.red() as f32 / 255.0 * inv_src_a).min(1.0);
                        let out_g =
                            (sg * coverage + dst.green() as f32 / 255.0 * inv_src_a).min(1.0);
                        let out_b =
                            (sb * coverage + dst.blue() as f32 / 255.0 * inv_src_a).min(1.0);
                        let out_a = (src_a + dst.alpha() as f32 / 255.0 * inv_src_a).min(1.0);

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::UiArena;
    use crate::layout::compute_layout;

    fn expected_width(text: &str, font_size: f32) -> f32 {
        let font = fonts::default_font();
        let scaled = font.as_scaled(fonts::scale(font_size));
        text.chars()
            .map(|c| scaled.h_advance(font.glyph_id(c)))
            .sum()
    }

    fn expected_height(font_size: f32) -> f32 {
        let font = fonts::default_font();
        let scaled = font.as_scaled(fonts::scale(font_size));
        scaled.height()
    }

    #[test]
    fn measure_matches_exact_glyph_metrics() {
        let mut arena = UiArena::<String>::new();
        let id = arena.spawn(Label::new("Hello").font_size(16.0));
        arena.set_root(id);

        let state = compute_layout(
            &arena,
            id,
            Size {
                width: 800.0,
                height: 600.0,
            },
            1.0,
        );
        let rect = state.get(id).expect("label frame");

        let ew = expected_width("Hello", 16.0);
        let eh = expected_height(16.0);
        assert!((rect.size.width - ew).abs() < 0.01, "width mismatch {ew}");
        assert!((rect.size.height - eh).abs() < 0.01, "height mismatch {eh}");
    }

    #[test]
    fn measure_larger_font() {
        let mut arena = UiArena::<String>::new();
        let id = arena.spawn(Label::new("AB").font_size(24.0));
        arena.set_root(id);

        let state = compute_layout(
            &arena,
            id,
            Size {
                width: 800.0,
                height: 600.0,
            },
            1.0,
        );
        let rect = state.get(id).expect("label frame");

        let ew = expected_width("AB", 24.0);
        let eh = expected_height(24.0);
        assert!((rect.size.width - ew).abs() < 0.01);
        assert!((rect.size.height - eh).abs() < 0.01);
    }

    #[test]
    fn measure_respects_constraints() {
        let mut arena = UiArena::<String>::new();
        let id = arena.spawn(Label::new("VeryLongText").font_size(16.0));
        arena.set_root(id);

        // Constrain the width to 50 px.
        let state = compute_layout(
            &arena,
            id,
            Size {
                width: 50.0,
                height: 50.0,
            },
            1.0,
        );
        let rect = state.get(id).expect("label frame");

        let ew = expected_width("VeryLongText", 16.0);
        // The unconstrained width is larger than 50 → should be clamped.
        assert!(ew > 50.0, "unconstrained width {ew} should be > 50");
        assert!((rect.size.width - 50.0).abs() < f32::EPSILON);
    }
}
