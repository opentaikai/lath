use tiny_skia::Color;

use crate::core::WidgetId;
use crate::layout::{Constraints, Point, Rect, Size};
use crate::widget::{Widget, WidgetMeasure};

/// A structural layout widget that wraps a single child with optional
/// padding and background colour.
///
/// # Layout
///
/// * **measure** – subtracts padding from incoming constraints, measures
///   the child (if any), then adds padding back to compute the final
///   desired size.
/// * **arrange** – positions the child at `(padding, padding)`.
/// * **draw** – fills the entire assigned `rect` with the background
///   colour (if one was set).
pub struct Container<M> {
    child: Option<WidgetId>,
    padding: f32,
    bg_color: Option<Color>,
    _marker: std::marker::PhantomData<M>,
}

impl<M> Container<M> {
    pub fn new() -> Self {
        Self {
            child: None,
            padding: 0.0,
            bg_color: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets the single child of this container.
    pub fn child(mut self, id: WidgetId) -> Self {
        self.child = Some(id);
        self
    }

    /// Sets the inner padding (applied on all four sides).
    pub fn padding(mut self, value: f32) -> Self {
        self.padding = value;
        self
    }

    /// Sets the background fill colour.
    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
        self
    }
}

impl<M> Default for Container<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Widget<M> for Container<M> {
    fn name(&self) -> &'static str {
        "Container"
    }

    fn children(&self) -> Vec<WidgetId> {
        self.child.into_iter().collect()
    }

    fn add_child(&mut self, child: WidgetId) {
        self.child = Some(child);
    }

    // -- Layout -------------------------------------------------------------

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
        if let Some(color) = self.bg_color {
            let mut paint = tiny_skia::Paint::default();
            paint.set_color(color);

            let skia_rect = tiny_skia::Rect::from_xywh(
                rect.origin.x,
                rect.origin.y,
                rect.size.width,
                rect.size.height,
            );
            if let Some(r) = skia_rect {
                canvas.fill_rect(r, &paint, tiny_skia::Transform::identity(), None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::UiArena;
    use crate::layout::compute_layout;

    /// A trivial leaf used only for layout tests.
    struct Leaf;

    impl Widget<String> for Leaf {
        fn name(&self) -> &'static str {
            "Leaf"
        }
        fn children(&self) -> Vec<WidgetId> {
            Vec::new()
        }
        fn measure(&self, _c: Constraints, _a: &dyn WidgetMeasure<String>) -> Size {
            Size {
                width: 50.0,
                height: 30.0,
            }
        }
        fn arrange(&self, _s: Size, _a: &dyn WidgetMeasure<String>) -> Vec<(WidgetId, Point)> {
            Vec::new()
        }
        fn draw(&self, _c: &mut tiny_skia::PixmapMut, _r: Rect) {}
    }

    #[test]
    fn container_measures_child_with_padding() {
        let mut arena = UiArena::<String>::new();
        let leaf = arena.spawn(Leaf);
        let root = arena.spawn(Container::<String>::new().padding(10.0).child(leaf));
        arena.set_root(root);

        let state = compute_layout(&arena, root, Size { width: 800.0, height: 600.0 });

        let root_rect = state.get(root).expect("root frame");
        // 50 + 10*2 = 70, 30 + 10*2 = 50
        assert_eq!(root_rect.size, Size { width: 70.0, height: 50.0 });

        let leaf_rect = state.get(leaf).expect("leaf frame");
        assert_eq!(leaf_rect.origin, Point { x: 10.0, y: 10.0 });
        assert_eq!(leaf_rect.size, Size { width: 50.0, height: 30.0 });
    }

    #[test]
    fn container_empty_has_zero_size() {
        let mut arena = UiArena::<String>::new();
        let root = arena.spawn(Container::<String>::new());
        arena.set_root(root);

        let state = compute_layout(&arena, root, Size { width: 800.0, height: 600.0 });
        let root_rect = state.get(root).expect("root frame");
        assert_eq!(root_rect.size, Size { width: 0.0, height: 0.0 });
    }
}
