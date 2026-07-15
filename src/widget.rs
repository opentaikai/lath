use crate::core::WidgetId;

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
}
