//! # Counter — Interactive `lath` Example
//!
//! Demonstrates all three built-in widgets (`Container`, `Label`, `Button`),
//! event dispatch via `UiContext`, and simple state management.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example counter
//! ```

use lath::events::{dispatch_event, UiContext};
use lath::layout::{compute_layout, Point, Size};
use lath::shell::{ShellEvent, WindowShell};
use lath::widgets::{Button, Container, Label};
use tiny_skia::Color;

// -- Message type -----------------------------------------------------------

/// Application messages emitted by interactive widgets.
#[derive(Clone, Default)]
enum Msg {
    #[default]
    NoOp,
    Increment,
}

// -- State + tree rebuild ---------------------------------------------------

/// Spawns a fresh widget tree reflecting the current `count`.
///
/// Because `Label` is immutable after creation, the simplest way to update
/// text is to rebuild the relevant subtree when state changes.  In a
/// production framework you would use a virtual DOM or mutable widget
/// slots; here we just respawn into the arena.
fn rebuild_tree(ui: &mut UiContext<Msg>, count: u32) {
    // Leaf: displays the current count.
    let count_label = ui.arena.spawn(
        Label::new(format!("Count: {count}"))
            .text_color(Color::from_rgba8(0x00, 0x00, 0x00, 0xFF))
            .font_size(24.0),
    );

    // Interactive: clicking emits Msg::Increment.
    let button = ui.arena.spawn(
        Button::new()
            .padding(16.0)
            .bg_color(Color::from_rgba8(0x40, 0x80, 0xF0, 0xFF))
            .child(count_label)
            .on_click(Msg::Increment),
    );

    // Structural: dark background + padding wrapping everything.
    let root = ui.arena.spawn(
        Container::new()
            .padding(40.0)
            .bg_color(Color::from_rgba8(0x1E, 0x1E, 0x1E, 0xFF))
            .child(button),
    );

    ui.arena.set_root(root);
}

// -- Main -------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = UiContext::<Msg>::new();
    let mut count: u32 = 0;
    let mut dirty = true; // force initial tree build

    let shell = WindowShell::new("lath — Counter", 400, 200)?;

    shell.run(move |event, pixmap, window| {
        // 1. Drain any messages emitted during the previous frame.
        for msg in ui.drain_events() {
            match msg {
                Msg::Increment => {
                    count = count.wrapping_add(1);
                    dirty = true;
                }
                Msg::NoOp => {}
            }
        }

        // 2. If state changed, rebuild the widget tree.
        if dirty {
            rebuild_tree(&mut ui, count);
            dirty = false;
        }

        let Some(root) = ui.arena.root() else {
            return;
        };

        // 3. Compute layout once for this event.
        //    The layout runs in logical points, then scales every rect
        //    to physical pixels — the LayoutState is ready for draw and
        //    hit-test alike.
        let scale_factor = match event {
            ShellEvent::Redraw { scale_factor } => scale_factor,
            ShellEvent::Resized { scale_factor, .. } => scale_factor,
            _ => 1.0, // fallback for other events
        };
        let logical_size = Size {
            width: window.inner_size().width as f32 / scale_factor,
            height: window.inner_size().height as f32 / scale_factor,
        };
        let layout = compute_layout(&ui.arena, root, logical_size, scale_factor);

        // 4. React to the current event.
        match event {
            // Click → hit-test and dispatch to the topmost widget.
            // Both click position (from the shell) and LayoutState rects
            // are in physical pixels, so they compare directly.
            ShellEvent::MouseButtonPressed { x, y, .. } => {
                dispatch_event(
                    &ui.arena,
                    &layout,
                    root,
                    Point {
                        x: x as f32,
                        y: y as f32,
                    },
                    &ui.event_ctx(),
                );
            }

            // Redraw → walk the tree and paint every widget.
            ShellEvent::Redraw { .. } => {
                ui.arena.traverse(root, |id, widget| {
                    if let Some(rect) = layout.get(id) {
                        widget.draw(pixmap, *rect);
                    }
                });
            }

            _ => {}
        }
    })?;

    Ok(())
}
