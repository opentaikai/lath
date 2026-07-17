//! # Interactive Text — Dynamic Text Resizing Demo
//!
//! Shows that layout invalidation works correctly when text changes:
//! clicking the button swaps between a short word and a long sentence.
//! The outer container expands and contracts fluidly to accommodate the
//! new text dimensions.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example interactive_text
//! ```

use lath::events::{dispatch_event, UiContext};
use lath::layout::{compute_layout, Point, Size};
use lath::shell::{ShellEvent, WindowShell};
use lath::widgets::{Button, Column, Container, Label};
use tiny_skia::Color;

#[derive(Clone, Default)]
enum Msg {
    #[default]
    None,
    Toggle,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = UiContext::<Msg>::new();
    let mut short = true;

    // Build initial tree.
    let label = ui.arena.spawn(
        Label::new("Hello")
            .text_color(Color::from_rgba8(0xEE, 0xEE, 0xEE, 0xFF))
            .font_size(24.0),
    );

    let inner = ui.arena.spawn(
        Container::new()
            .padding(16.0)
            .bg_color(Color::from_rgba8(0x2D, 0x2D, 0x2D, 0xFF))
            .child(label),
    );

    let btn_label = ui.arena.spawn(
        Label::new("Toggle Text")
            .text_color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF))
            .font_size(14.0),
    );

    let button = ui.arena.spawn(
        Button::new()
            .padding(10.0)
            .bg_color(Color::from_rgba8(0x40, 0x80, 0xF0, 0xFF))
            .child(btn_label)
            .on_click(Msg::Toggle),
    );

    let root = ui
        .arena
        .spawn(Column::new().spacing(20.0).child(inner).child(button));

    ui.arena.set_root(root);

    // Store the container ID so we can replace its child later.
    // (We also need inner and label, but container's add_child replaces.)
    let container_id = inner;

    // -----------------------------------------------------------------------
    // Run the window
    // -----------------------------------------------------------------------

    let shell = WindowShell::new("lath — Interactive Text", 400, 200)?;

    shell.run(move |event, pixmap, window| {
        // 1. Drain messages into a local vec to release the borrow on `ui`.
        let messages: Vec<_> = ui.drain_events().collect();
        for msg in messages {
            if let Msg::Toggle = msg {
                short = !short;
                let new_text = if short {
                    "Hello"
                } else {
                    "The quick brown fox jumps over the lazy dog"
                };

                // Spawn a new label with the updated text and swap it into
                // the container via add_child.
                let new_label = ui.arena.spawn(
                    Label::new(new_text)
                        .text_color(Color::from_rgba8(0xEE, 0xEE, 0xEE, 0xFF))
                        .font_size(24.0),
                );

                if let Some(container) = ui.arena.get_mut(container_id) {
                    container.add_child(new_label);
                }

                window.request_redraw();
            }
        }

        let Some(root) = ui.arena.root() else {
            return;
        };

        // 2. Compute layout for the current event.
        let scale_factor = match event {
            ShellEvent::Redraw { scale_factor } | ShellEvent::Resized { scale_factor, .. } => {
                scale_factor
            }
            _ => 1.0,
        };
        let logical_size = Size {
            width: window.inner_size().width as f32 / scale_factor,
            height: window.inner_size().height as f32 / scale_factor,
        };
        let layout = compute_layout(&ui.arena, root, logical_size, scale_factor);

        // 3. React to the current event.
        match event {
            ShellEvent::MouseButtonPressed { x, y, .. } => {
                // Dispatch the click to trigger the button's handle_event,
                // which sends Msg::Toggle into the channel.
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
