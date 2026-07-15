use crate::core::WidgetId;
use crate::layout::{Constraints, Point, Size};

/// Base trait for all UI widgets in the lath framework.
///
/// Widgets are stored as trait objects inside [`crate::core::UiArena`].
/// Each widget owns its children by `WidgetId` reference rather than
/// by pointer, keeping the storage flat and the borrow rules simple.
///
/// The generic parameter `M` is the application-level message type
/// produced by event handling (Elm-style architecture).
pub trait Widget<M> {
    /// A human-readable name for this widget type (e.g. `"Button"`).
    fn name(&self) -> &'static str;

    /// Returns the `WidgetId`s of the immediate children.
    fn children(&self) -> Vec<WidgetId>;

    /// Adds a child to this widget. Only meaningful for container widgets;
    /// the default implementation is a no-op.
    fn add_child(&mut self, _child: WidgetId) {}

    // -- Layout hooks -------------------------------------------------------

    /// **Pass 1 (measure):** Returns the preferred size of this widget given
    /// the incoming `constraints`.  Called bottom-up: a parent's `measure`
    /// implementation should call `measure` on its children through the
    /// arena to discover their sizes before deciding its own.
    ///
    /// The default implementation fills the maximum available space.
    fn measure(&self, constraints: Constraints, _arena: &dyn WidgetMeasure<M>) -> Size {
        constraints.constrain(Size {
            width: constraints.max_width,
            height: constraints.max_height,
        })
    }

    /// **Pass 2 (arrange):** Given the concrete `size` that has been
    /// allocated to this widget, return the list of children together with
    /// their **local** offset (relative to this widget's origin).
    ///
    /// The solver translates these local offsets into absolute screen-space
    /// coordinates.  The default implementation positions nothing.
    fn arrange(&self, _size: Size, _arena: &dyn WidgetMeasure<M>) -> Vec<(WidgetId, Point)> {
        Vec::new()
    }
}

/// Minimal read-only view of the arena that layout hooks may use to
/// query sibling/child sizes without coupling to the full `UiArena` type.
///
/// Implemented automatically for anything that can look up a `WidgetId`
/// and return a widget reference.
pub trait WidgetMeasure<M> {
    fn get_widget(&self, id: WidgetId) -> Option<&(dyn Widget<M> + '_)>;
}
