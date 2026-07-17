use crate::core::WidgetId;
use crate::layout::{Constraints, Point, Rect, Size};
use crate::widget::{Widget, WidgetMeasure};

/// A structural layout widget that stacks its children vertically
/// (top to bottom).  Purely geometric — no visual of its own.
///
/// # Layout
///
/// * **measure** – sums each child's height plus `spacing` gaps;
///   width is the widest child.  Clamped to incoming constraints.
/// * **arrange** – walks children top→bottom with a running y-offset.
/// * **draw** – no-op (children are painted by the tree walker).
pub struct Column {
    children: Vec<WidgetId>,
    spacing: f32,
}

impl Column {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            spacing: 0.0,
        }
    }

    /// Sets the gap (in logical points) between consecutive children.
    pub fn spacing(mut self, value: f32) -> Self {
        self.spacing = value;
        self
    }

    /// Appends a child to the end of the stack.
    pub fn child(mut self, id: WidgetId) -> Self {
        self.children.push(id);
        self
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Widget<M> for Column {
    fn name(&self) -> &'static str {
        "Column"
    }

    fn children(&self) -> Vec<WidgetId> {
        self.children.clone()
    }

    fn add_child(&mut self, child: WidgetId) {
        self.children.push(child);
    }

    // -- Layout -------------------------------------------------------------

    fn measure(&self, constraints: Constraints, arena: &dyn WidgetMeasure<M>) -> Size {
        let mut total_height = 0.0_f32;
        let mut max_width = 0.0_f32;
        let mut first = true;

        for &child_id in &self.children {
            if first {
                first = false;
            } else {
                total_height += self.spacing;
            }

            if let Some(child) = arena.get_widget(child_id) {
                let child_constraints = Constraints::loose(constraints.max_width, f32::INFINITY);
                let child_size = child.measure(child_constraints, arena);
                total_height += child_size.height;
                max_width = max_width.max(child_size.width);
            }
        }

        constraints.constrain(Size {
            width: max_width,
            height: total_height,
        })
    }

    fn arrange(&self, _size: Size, arena: &dyn WidgetMeasure<M>) -> Vec<(WidgetId, Point)> {
        let mut offsets = Vec::new();
        let mut current_y = 0.0_f32;
        let mut first = true;

        for &child_id in &self.children {
            if first {
                first = false;
            } else {
                current_y += self.spacing;
            }

            let child_size = arena
                .get_widget(child_id)
                .map(|c| c.measure(Constraints::loose(f32::INFINITY, f32::INFINITY), arena))
                .unwrap_or(Size::ZERO);

            offsets.push((
                child_id,
                Point {
                    x: 0.0,
                    y: current_y,
                },
            ));

            current_y += child_size.height;
        }

        offsets
    }

    // -- Drawing / interaction (purely structural) ---------------------------

    fn draw(&self, _canvas: &mut tiny_skia::PixmapMut, _rect: Rect) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::UiArena;
    use crate::layout::compute_layout;

    /// A leaf with a fixed intrinsic size.
    struct Fixed {
        intrinsic: Size,
    }

    impl Widget<String> for Fixed {
        fn name(&self) -> &'static str {
            "Fixed"
        }
        fn children(&self) -> Vec<WidgetId> {
            Vec::new()
        }
        fn measure(&self, c: Constraints, _: &dyn WidgetMeasure<String>) -> Size {
            c.constrain(self.intrinsic)
        }
        fn arrange(&self, _: Size, _: &dyn WidgetMeasure<String>) -> Vec<(WidgetId, Point)> {
            Vec::new()
        }
        fn draw(&self, _: &mut tiny_skia::PixmapMut, _: Rect) {}
    }

    #[test]
    fn column_stacks_vertically() {
        let mut arena = UiArena::<String>::new();
        let a = arena.spawn(Fixed {
            intrinsic: Size {
                width: 100.0,
                height: 30.0,
            },
        });
        let b = arena.spawn(Fixed {
            intrinsic: Size {
                width: 80.0,
                height: 40.0,
            },
        });
        let root = arena.spawn(Column::new().spacing(10.0).child(a).child(b));
        arena.set_root(root);

        let state = compute_layout(
            &arena,
            root,
            Size {
                width: 800.0,
                height: 600.0,
            },
            1.0,
        );

        let root_rect = state.get(root).expect("root frame");
        // Widest child = 100.  Heights = 30 + 10 + 40 = 80.
        assert_eq!(
            root_rect.size,
            Size {
                width: 100.0,
                height: 80.0
            }
        );

        let a_rect = state.get(a).expect("child a");
        assert_eq!(a_rect.origin, Point { x: 0.0, y: 0.0 });

        let b_rect = state.get(b).expect("child b");
        // b starts at y = 30 (a's height) + 10 (spacing) = 40.
        assert_eq!(b_rect.origin, Point { x: 0.0, y: 40.0 });
    }

    #[test]
    fn column_empty_returns_zero() {
        let mut arena = UiArena::<String>::new();
        let root = arena.spawn(Column::new());
        arena.set_root(root);

        let state = compute_layout(
            &arena,
            root,
            Size {
                width: 800.0,
                height: 600.0,
            },
            1.0,
        );

        let r = state.get(root).expect("root frame");
        assert_eq!(r.size, Size::ZERO);
    }
}
