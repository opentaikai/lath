use tiny_skia::Color;

use crate::core::WidgetId;
use crate::layout::{Constraints, Point, Rect, Size};
use crate::widget::{EventCtx, Widget, WidgetEvent, WidgetMeasure};

/// An interactive container that wraps a single child and optionally
/// produces a message `M` when clicked.
///
/// # Layout
///
/// Behaves identically to [`Container`]: subtracts padding from the
/// incoming constraints, measures the child, and adds padding back.
///
/// # Drawing
///
/// Fills the entire assigned `rect` with `bg_color` (defaulting to a
/// neutral grey) and then delegates rendering to its child.
pub struct Button<M> {
    child: Option<WidgetId>,
    padding: f32,
    bg_color: Color,
    on_click: Option<M>,
}

impl<M> Button<M> {
    pub fn new() -> Self
    where
        M: Default,
    {
        Self {
            child: None,
            padding: 8.0,
            bg_color: Color::from_rgba8(0xE0, 0xE0, 0xE0, 0xFF),
            on_click: None,
        }
    }

    /// Sets the single child of this button (typically a [`Label`]).
    pub fn child(mut self, id: WidgetId) -> Self {
        self.child = Some(id);
        self
    }

    /// Sets the inner padding.
    pub fn padding(mut self, value: f32) -> Self {
        self.padding = value;
        self
    }

    /// Sets the background fill colour.
    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    /// Sets the message that will be emitted when the button is clicked.
    pub fn on_click(mut self, msg: M) -> Self {
        self.on_click = Some(msg);
        self
    }

    /// Returns a reference to the stored click message, if any.
    pub fn click_message(&self) -> Option<&M> {
        self.on_click.as_ref()
    }
}

impl<M> Default for Button<M>
where
    M: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Clone> Widget<M> for Button<M> {
    fn name(&self) -> &'static str {
        "Button"
    }

    fn children(&self) -> Vec<WidgetId> {
        self.child.into_iter().collect()
    }

    fn add_child(&mut self, child: WidgetId) {
        self.child = Some(child);
    }

    // -- Layout (mirrors Container) -----------------------------------------

    fn measure(&self, constraints: Constraints, arena: &dyn WidgetMeasure<M>) -> Size {
        let inset = self.padding * 2.0;

        let inner = match self.child {
            Some(id) => {
                let child_constraints = Constraints::loose(
                    (constraints.max_width - inset).max(0.0),
                    (constraints.max_height - inset).max(0.0),
                );
                if let Some(child) = arena.get_widget(id) {
                    child.measure(child_constraints, arena)
                } else {
                    Size::ZERO
                }
            }
            None => Size::ZERO,
        };

        constraints.constrain(Size {
            width: inner.width + inset,
            height: inner.height + inset,
        })
    }

    fn arrange(&self, _size: Size, _arena: &dyn WidgetMeasure<M>) -> Vec<(WidgetId, Point)> {
        match self.child {
            Some(id) => vec![(id, Point {
                x: self.padding,
                y: self.padding,
            })],
            None => Vec::new(),
        }
    }

    // -- Drawing ------------------------------------------------------------

    fn draw(&self, canvas: &mut tiny_skia::PixmapMut, rect: Rect) {
        // Draw the button background.
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(self.bg_color);

        if let Some(r) = tiny_skia::Rect::from_xywh(
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        ) {
            canvas.fill_rect(r, &paint, tiny_skia::Transform::identity(), None);
        }

        // Note: child drawing is handled by the render traversal, not here.
        // The button only draws its own background; the child draws itself.
    }

    // -- Interaction --------------------------------------------------------

    fn handle_event(&self, event: WidgetEvent, ctx: &EventCtx<M>) -> bool {
        if let WidgetEvent::Click = event {
            if let Some(msg) = &self.on_click {
                ctx.emit(msg.clone());
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::UiArena;
    use crate::layout::compute_layout;
    use crate::widgets::Label;

    #[test]
    fn button_measures_child_with_padding() {
        let mut arena = UiArena::<String>::new();
        let label = arena.spawn(Label::new("OK"));
        let btn = arena.spawn(Button::<String>::new().padding(12.0).child(label));
        arena.set_root(btn);

        let state = compute_layout(&arena, btn, Size { width: 800.0, height: 600.0 }, 1.0);

        let btn_rect = state.get(btn).expect("button frame");
        // Label "OK" = 2 × 16 × 0.6 = 19.2, height 16.0
        // With padding 12: 19.2 + 24 = 43.2, 16.0 + 24 = 40.0
        assert!((btn_rect.size.width - 43.2).abs() < f32::EPSILON);
        assert!((btn_rect.size.height - 40.0).abs() < f32::EPSILON);

        let label_rect = state.get(label).expect("label frame");
        assert_eq!(label_rect.origin, Point { x: 12.0, y: 12.0 });
    }

    #[test]
    fn button_default_has_grey_background() {
        let btn = Button::<String>::new();
        assert_eq!(btn.bg_color, Color::from_rgba8(0xE0, 0xE0, 0xE0, 0xFF));
    }

    #[test]
    fn button_stores_click_message() {
        let btn = Button::new().on_click("clicked".to_string());
        assert_eq!(btn.click_message(), Some(&"clicked".to_string()));
    }
}
