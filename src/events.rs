use std::sync::mpsc::{self, Receiver, Sender};

use crate::core::{UiArena, WidgetId};
use crate::layout::{LayoutState, Point, Rect};
use crate::widget::{EventCtx, WidgetEvent};

// ---------------------------------------------------------------------------
// Hit-test & dispatch
// ---------------------------------------------------------------------------

/// Performs a top-down, front-to-back hit-test on the widget tree and
/// dispatches a [`WidgetEvent::Click`] to the first interactive widget
/// whose bounds contain `click_pos`.
///
/// Propagation stops as soon as one widget handles the event, preventing
/// underlying containers from also receiving the click.
pub fn dispatch_event<M>(
    arena: &UiArena<M>,
    layout: &LayoutState,
    root: WidgetId,
    click_pos: Point,
    event_ctx: &EventCtx<M>,
) {
    hit_test_recursive(arena, layout, root, click_pos, event_ctx);
}

/// Recursive helper.  Returns `true` if a widget handled the event
/// (propagation should stop).
fn hit_test_recursive<M>(
    arena: &UiArena<M>,
    layout: &LayoutState,
    id: WidgetId,
    click_pos: Point,
    event_ctx: &EventCtx<M>,
) -> bool {
    let widget = match arena.get(id) {
        Some(w) => w,
        None => return false,
    };

    // Visit children first – they render on top of the parent, so
    // they should be hit-tested first (front-to-back).
    let children = widget.children();
    for &child_id in &children {
        if hit_test_recursive(arena, layout, child_id, click_pos, event_ctx) {
            return true;
        }
    }

    // Check whether the click falls inside this widget's bounds.
    if let Some(rect) = layout.get(id) {
        if point_in_rect(click_pos, rect) {
            widget.handle_event(WidgetEvent::Click, event_ctx);
            return true;
        }
    }

    false
}

/// Returns `true` when `point` lies inside `rect` (top-left inclusive,
/// bottom-right exclusive).
fn point_in_rect(point: Point, rect: &Rect) -> bool {
    point.x >= rect.origin.x
        && point.x < rect.right()
        && point.y >= rect.origin.y
        && point.y < rect.bottom()
}

// ---------------------------------------------------------------------------
// UiContext – arena + event channel
// ---------------------------------------------------------------------------

/// Top-level context that owns the widget arena and the event channel.
///
/// Create one at application startup, spawn widgets into the arena,
/// and call [`drain_events`](Self::drain_events) each frame to consume
/// messages produced by interactive widgets.
pub struct UiContext<M> {
    /// The widget arena (public so callers can spawn / query widgets).
    pub arena: UiArena<M>,
    rx: Receiver<M>,
    tx: Sender<M>,
}

impl<M> UiContext<M> {
    /// Creates a fresh context with an empty arena and a new channel.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            arena: UiArena::new(),
            rx,
            tx,
        }
    }

    /// Returns an [`EventCtx`] that can be handed to [`dispatch_event`]
    /// or to individual widget `handle_event` calls.
    pub fn event_ctx(&self) -> EventCtx<M> {
        EventCtx {
            tx: self.tx.clone(),
        }
    }

    /// Drains all pending messages from the event channel.
    ///
    /// Typical usage inside the application loop:
    ///
    /// ```ignore
    /// for msg in ui_ctx.drain_events() {
    ///     match msg {
    ///         AppMsg::ButtonClicked => { /* update state */ }
    ///     }
    /// }
    /// ```
    pub fn drain_events(&self) -> std::sync::mpsc::TryIter<'_, M> {
        self.rx.try_iter()
    }
}

impl<M> Default for UiContext<M> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{compute_layout, Size};
    use crate::widgets::{Button, Label};

    #[test]
    fn click_button_emits_message() {
        #[derive(Debug, Clone, Default, PartialEq)]
        enum Msg {
            #[default]
            Clicked,
            Other,
        }

        let mut ui = UiContext::<Msg>::new();

        let label = ui.arena.spawn(Label::new("OK"));
        let btn = ui.arena.spawn(
            Button::new()
                .padding(8.0)
                .child(label)
                .on_click(Msg::Clicked),
        );
        ui.arena.set_root(btn);

        let state = compute_layout(
            &ui.arena,
            btn,
            Size {
                width: 800.0,
                height: 600.0,
            },
        );

        let btn_rect = state.get(btn).expect("button should have a frame");

        // Click in the centre of the button.
        let center = Point {
            x: btn_rect.origin.x + btn_rect.size.width / 2.0,
            y: btn_rect.origin.y + btn_rect.size.height / 2.0,
        };

        dispatch_event(&ui.arena, &state, btn, center, &ui.event_ctx());

        let messages: Vec<_> = ui.drain_events().collect();
        assert_eq!(messages, vec![Msg::Clicked]);
    }

    #[test]
    fn click_misses_non_interactive_widget() {
        #[derive(Debug, Clone, Default, PartialEq)]
        enum Msg {
            #[default]
            Clicked,
        }

        let mut ui = UiContext::<Msg>::new();

        // A plain Label is non-interactive (handle_event is a no-op).
        let id = ui.arena.spawn(Label::new("Hi"));
        ui.arena.set_root(id);

        let state = compute_layout(
            &ui.arena,
            id,
            Size {
                width: 800.0,
                height: 600.0,
            },
        );

        let rect = state.get(id).expect("label frame");
        let center = Point {
            x: rect.origin.x + rect.size.width / 2.0,
            y: rect.origin.y + rect.size.height / 2.0,
        };

        dispatch_event(&ui.arena, &state, id, center, &ui.event_ctx());

        let messages: Vec<_> = ui.drain_events().collect();
        assert!(messages.is_empty());
    }

    #[test]
    fn click_child_takes_priority_over_parent() {
        #[derive(Debug, Clone, Default, PartialEq)]
        enum Msg {
            #[default]
            ParentClicked,
            ChildClicked,
        }

        let mut ui = UiContext::<Msg>::new();

        let inner_label = ui.arena.spawn(Label::new("X"));
        let child_btn = ui.arena.spawn(
            Button::new()
                .padding(4.0)
                .child(inner_label)
                .on_click(Msg::ChildClicked),
        );

        let parent_label = ui.arena.spawn(Label::new("P"));
        let parent_btn = ui.arena.spawn(
            Button::new()
                .padding(0.0)
                .child(child_btn)
                .on_click(Msg::ParentClicked),
        );
        ui.arena.set_root(parent_btn);

        let state = compute_layout(
            &ui.arena,
            parent_btn,
            Size {
                width: 800.0,
                height: 600.0,
            },
        );

        // Click inside the child's bounds.
        let child_rect = state.get(child_btn).expect("child frame");
        let center = Point {
            x: child_rect.origin.x + child_rect.size.width / 2.0,
            y: child_rect.origin.y + child_rect.size.height / 2.0,
        };

        dispatch_event(&ui.arena, &state, parent_btn, center, &ui.event_ctx());

        let messages: Vec<_> = ui.drain_events().collect();
        assert_eq!(messages, vec![Msg::ChildClicked]);
    }

    #[test]
    fn click_outside_dispatches_nothing() {
        #[derive(Debug, Clone, Default, PartialEq)]
        enum Msg {
            #[default]
            Clicked,
        }

        let mut ui = UiContext::<Msg>::new();

        let inner_label = ui.arena.spawn(Label::new("Btn"));
        let btn = ui.arena.spawn(
            Button::new()
                .padding(4.0)
                .child(inner_label)
                .on_click(Msg::Clicked),
        );
        ui.arena.set_root(btn);

        let state = compute_layout(
            &ui.arena,
            btn,
            Size {
                width: 800.0,
                height: 600.0,
            },
        );

        // Click far away from the button.
        let outside = Point {
            x: 9999.0,
            y: 9999.0,
        };

        dispatch_event(&ui.arena, &state, btn, outside, &ui.event_ctx());

        let messages: Vec<_> = ui.drain_events().collect();
        assert!(messages.is_empty());
    }
}
