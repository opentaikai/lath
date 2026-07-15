use tiny_skia::Color;

use crate::core::WidgetId;
use crate::layout::{Constraints, Point, Rect, Size};
use crate::widget::{Widget, WidgetMeasure};

/// A static, single-line text renderer.
///
/// # Layout
///
/// * **measure** – approximates the text bounding box:
///   `width = char_count × font_size × 0.6`, `height = font_size`.
/// * **arrange** – leaf node, returns nothing.
/// * **draw** – renders a coloured rectangle representing the text
///   bounding box.  (Full glyph rendering will replace this once a
///   font rasteriser is integrated.)
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
        let width = self.text.len() as f32 * char_width;
        let height = self.font_size;

        constraints.constrain(Size { width, height })
    }

    fn arrange(&self, _size: Size, _arena: &dyn WidgetMeasure<M>) -> Vec<(WidgetId, Point)> {
        Vec::new()
    }

    // -- Drawing ------------------------------------------------------------

    fn draw(&self, canvas: &mut tiny_skia::PixmapMut, rect: Rect) {
        // MVP: draw a filled rectangle in the text colour to represent the
        // text bounding box.  A future integration with ab_glyph / cosmic-text
        // will replace this with actual glyph rasterisation.
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(self.text_color);

        if let Some(r) = tiny_skia::Rect::from_xywh(
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        ) {
            canvas.fill_rect(r, &paint, tiny_skia::Transform::identity(), None);
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
