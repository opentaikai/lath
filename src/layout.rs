use std::collections::HashMap;

use crate::core::{UiArena, WidgetId};
use crate::widget::{Widget, WidgetMeasure};

// ---------------------------------------------------------------------------
// Geometry primitives
// ---------------------------------------------------------------------------

/// A 2D size with floating-point components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };
}

/// A 2D point with floating-point components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
}

/// An axis-aligned rectangle defined by an origin and a size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    /// The right edge (`origin.x + size.width`).
    pub fn right(&self) -> f32 {
        self.origin.x + self.size.width
    }

    /// The bottom edge (`origin.y + size.height`).
    pub fn bottom(&self) -> f32 {
        self.origin.y + self.size.height
    }
}

/// Layout constraints that bound a widget's allowed size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Constraints {
    /// Creates constraints with no minimum and the given maximum.
    pub fn loose(max_width: f32, max_height: f32) -> Self {
        Self {
            min_width: 0.0,
            max_width,
            min_height: 0.0,
            max_height,
        }
    }

    /// Creates constraints that enforce an exact size.
    pub fn tight(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }

    /// Clamps `size` to lie within these constraints.
    pub fn constrain(&self, size: Size) -> Size {
        Size {
            width: size.width.clamp(self.min_width, self.max_width),
            height: size.height.clamp(self.min_height, self.max_height),
        }
    }
}

// ---------------------------------------------------------------------------
// LayoutState
// ---------------------------------------------------------------------------

/// Stores the final computed layout (position + size) for every widget
/// that participated in a layout pass.
#[derive(Debug, Clone)]
pub struct LayoutState {
    frames: HashMap<WidgetId, Rect>,
}

impl LayoutState {
    pub fn new() -> Self {
        Self {
            frames: HashMap::new(),
        }
    }

    /// Returns the computed `Rect` for the given widget, if it was laid out.
    pub fn get(&self, id: WidgetId) -> Option<&Rect> {
        self.frames.get(&id)
    }

    /// Returns `true` if the widget has a computed layout.
    pub fn contains(&self, id: WidgetId) -> bool {
        self.frames.contains_key(&id)
    }

    /// Clears all stored frames.
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

impl Default for LayoutState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// WidgetMeasure impl for UiArena
// ---------------------------------------------------------------------------

impl<M> WidgetMeasure<M> for UiArena<M> {
    fn get_widget(&self, id: WidgetId) -> Option<&(dyn Widget<M> + '_)> {
        self.get(id)
    }
}

// ---------------------------------------------------------------------------
// Solver
// ---------------------------------------------------------------------------

/// Computes the layout for the entire widget tree rooted at `root`.
///
/// # Algorithm
///
/// 1. **Measure pass** – calls `root.measure()` with loose constraints
///    derived from `window_size`.  Widgets may recursively measure their
///    children through the arena to determine their own preferred size.
///
/// 2. **Arrange pass** – walks the tree top-down.  Each widget's `arrange`
///    method returns child offsets (local to the parent).  The solver
///    converts these to absolute screen-space coordinates, measures the
///    child to obtain its size, records the child's `Rect` in the
///    [`LayoutState`], and recurses.
pub fn compute_layout<M>(
    arena: &UiArena<M>,
    root: WidgetId,
    window_size: Size,
) -> LayoutState {
    let mut state = LayoutState::new();

    let root_widget = match arena.get(root) {
        Some(w) => w,
        None => return state,
    };

    // Pass 1: measure root.
    let constraints = Constraints::loose(window_size.width, window_size.height);
    let root_size = root_widget.measure(constraints, arena);

    let root_rect = Rect::new(Point::ZERO, root_size);
    state.frames.insert(root, root_rect);

    // Pass 2: arrange recursively.
    arrange_children(arena, root, root_size, Point::ZERO, &mut state);

    state
}

/// Arranges the children of `parent_id`, converting local offsets to
/// absolute coordinates and recording each child's `Rect`.
fn arrange_children<M>(
    arena: &UiArena<M>,
    parent_id: WidgetId,
    parent_size: Size,
    parent_origin: Point,
    state: &mut LayoutState,
) {
    let parent_widget = match arena.get(parent_id) {
        Some(w) => w,
        None => return,
    };

    let local_offsets = parent_widget.arrange(parent_size, arena);

    for (child_id, local_offset) in local_offsets {
        let child_widget = match arena.get(child_id) {
            Some(w) => w,
            None => continue,
        };

        let absolute_x = parent_origin.x + local_offset.x;
        let absolute_y = parent_origin.y + local_offset.y;

        // Measure the child to discover its size.
        let child_constraints = Constraints::loose(parent_size.width, parent_size.height);
        let child_size = child_widget.measure(child_constraints, arena);

        let child_rect = Rect::new(
            Point {
                x: absolute_x,
                y: absolute_y,
            },
            child_size,
        );
        state.frames.insert(child_id, child_rect);

        // Recurse into the child's own children.
        arrange_children(arena, child_id, child_size, child_rect.origin, state);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::UiArena;

    // -- Mock widgets -------------------------------------------------------

    /// A leaf widget with a fixed intrinsic size.
    struct FixedBox {
        label: &'static str,
        intrinsic: Size,
    }

    impl FixedBox {
        fn new(label: &'static str, width: f32, height: f32) -> Self {
            Self {
                label,
                intrinsic: Size {
                    width,
                    height,
                },
            }
        }
    }

    impl Widget<String> for FixedBox {
        fn name(&self) -> &'static str {
            self.label
        }

        fn children(&self) -> Vec<WidgetId> {
            Vec::new()
        }

        fn measure(&self, constraints: Constraints, _arena: &dyn WidgetMeasure<String>) -> Size {
            constraints.constrain(self.intrinsic)
        }

        fn arrange(&self, _size: Size, _arena: &dyn WidgetMeasure<String>) -> Vec<(WidgetId, Point)> {
            Vec::new()
        }

        fn draw(&self, _canvas: &mut tiny_skia::PixmapMut, _rect: Rect) {}
    }

    /// A container that stacks children vertically, each at full parent width.
    struct VStack {
        children: Vec<WidgetId>,
    }

    impl VStack {
        fn new() -> Self {
            Self {
                children: Vec::new(),
            }
        }
    }

    impl Widget<String> for VStack {
        fn name(&self) -> &'static str {
            "VStack"
        }

        fn children(&self) -> Vec<WidgetId> {
            self.children.clone()
        }

        fn add_child(&mut self, child: WidgetId) {
            self.children.push(child);
        }

        fn measure(&self, constraints: Constraints, arena: &dyn WidgetMeasure<String>) -> Size {
            let mut total_height: f32 = 0.0;
            let mut max_width: f32 = 0.0;

            for &child_id in &self.children {
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

        fn arrange(&self, size: Size, _arena: &dyn WidgetMeasure<String>) -> Vec<(WidgetId, Point)> {
            let mut offsets = Vec::new();
            let mut y: f32 = 0.0;

            for &child_id in &self.children {
                offsets.push((child_id, Point { x: 0.0, y }));
                // Vertical stacking: advance y by a default child height.
                // In a real framework the parent would know the child's size
                // from a measure cache; here we approximate.
                y += size.height / self.children.len() as f32;
            }

            offsets
        }

        fn draw(&self, _canvas: &mut tiny_skia::PixmapMut, _rect: Rect) {}
    }

    // -- Tests --------------------------------------------------------------

    #[test]
    fn single_fixed_child() {
        let mut arena = UiArena::<String>::new();
        let child = arena.spawn(FixedBox::new("Box", 120.0, 80.0));
        let root = arena.spawn(VStack::new());
        arena.get_mut(root).unwrap().add_child(child);
        arena.set_root(root);

        let state = compute_layout(&arena, root, Size { width: 800.0, height: 600.0 });

        // Root wraps its child (VStack is a wrapping layout, not a filling one).
        let root_rect = state.get(root).expect("root should have a frame");
        assert_eq!(root_rect.size, Size { width: 120.0, height: 80.0 });

        // Child is placed at the top-left of the root with its intrinsic size.
        let child_rect = state.get(child).expect("child should have a frame");
        assert_eq!(child_rect.origin, Point { x: 0.0, y: 0.0 });
        assert_eq!(child_rect.size, Size { width: 120.0, height: 80.0 });
    }

    #[test]
    fn nested_vstacks() {
        // Root (VStack)
        //   ├─ A (FixedBox 100×40)
        //   └─ B (VStack)
        //       ├─ B1 (FixedBox 60×30)
        //       └─ B2 (FixedBox 60×50)
        let mut arena = UiArena::<String>::new();

        let a = arena.spawn(FixedBox::new("A", 100.0, 40.0));
        let b1 = arena.spawn(FixedBox::new("B1", 60.0, 30.0));
        let b2 = arena.spawn(FixedBox::new("B2", 60.0, 50.0));
        let b = arena.spawn(VStack::new());
        let root = arena.spawn(VStack::new());

        arena.get_mut(b).unwrap().add_child(b1);
        arena.get_mut(b).unwrap().add_child(b2);
        arena.get_mut(root).unwrap().add_child(a);
        arena.get_mut(root).unwrap().add_child(b);
        arena.set_root(root);

        let state = compute_layout(&arena, root, Size { width: 400.0, height: 300.0 });

        // Root should exist.
        assert!(state.contains(root));

        // A is the first child of root.
        let a_rect = state.get(a).expect("A should have a frame");
        assert_eq!(a_rect.origin, Point { x: 0.0, y: 0.0 });

        // B is the second child.
        let b_rect = state.get(b).expect("B should have a frame");
        assert_eq!(b_rect.origin.x, 0.0);
        // B's y offset is determined by the VStack arrange logic.
        assert!(b_rect.origin.y > 0.0);

        // B1 and B2 exist in the layout.
        assert!(state.contains(b1));
        assert!(state.contains(b2));
    }

    #[test]
    fn constraints_clamp_size() {
        let c = Constraints::loose(100.0, 100.0);
        let big = Size {
            width: 500.0,
            height: 500.0,
        };
        assert_eq!(c.constrain(big), Size { width: 100.0, height: 100.0 });

        let small = Size {
            width: 10.0,
            height: 10.0,
        };
        assert_eq!(c.constrain(small), Size { width: 10.0, height: 10.0 });
    }

    #[test]
    fn tight_constraints() {
        let c = Constraints::tight(200.0, 150.0);
        let any = Size {
            width: 999.0,
            height: 1.0,
        };
        assert_eq!(c.constrain(any), Size { width: 200.0, height: 150.0 });
    }

    #[test]
    fn rect_edges() {
        let r = Rect::new(
            Point { x: 10.0, y: 20.0 },
            Size {
                width: 100.0,
                height: 50.0,
            },
        );
        assert_eq!(r.right(), 110.0);
        assert_eq!(r.bottom(), 70.0);
    }
}
