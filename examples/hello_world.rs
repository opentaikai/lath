//! # Hello, World! — Minimal `lath` Example
//!
//! This example demonstrates the absolute minimum boilerplate needed to:
//!
//! 1. Create a window with `WindowShell`.
//! 2. Build a static widget tree using `Container` and `Label`.
//! 3. Compute layout and draw every frame.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example hello_world
//! ```

use lath::core::UiArena;
use lath::layout::{compute_layout, Size};
use lath::shell::{ShellEvent, WindowShell};
use lath::widgets::{Container, Label};
use tiny_skia::Color;

// An uninhabitable enum used as the message type `M`.
// Since this example has no interactive widgets (no buttons), we never
// need to produce or handle messages.  `Void` satisfies the generic
// parameter at zero cost.
enum Void {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -----------------------------------------------------------------------
    // 1. Build the widget tree
    // -----------------------------------------------------------------------

    // The arena stores all widgets in a flat Vec, referenced by lightweight
    // `WidgetId` handles — no smart pointers or lifetimes involved.
    let mut arena = UiArena::<Void>::new();

    // Spawn a Label — a leaf widget that renders text (currently as a
    // coloured rectangle; real glyph rendering is coming later).
    let label = arena.spawn(
        Label::new("Hello, World from Lath!")
            .text_color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)) // white text
            .font_size(28.0),
    );

    // Spawn a Container — a structural wrapper that adds padding and an
    // optional background colour around its single child.
    let root = arena.spawn(
        Container::new()
            .padding(40.0)
            .bg_color(Color::from_rgba8(0x1E, 0x1E, 0x1E, 0xFF)) // charcoal
            .child(label),
    );

    // Tell the arena which widget is the tree root.  The layout solver
    // and the draw traversal both start from this id.
    arena.set_root(root);

    // -----------------------------------------------------------------------
    // 2. Create the window and run the event loop
    // -----------------------------------------------------------------------

    let shell = WindowShell::new("lath — Hello World", 800, 600)?;

    // `run` consumes the shell and blocks until the window is closed.
    // The closure is invoked for every platform event (redraw, resize,
    // mouse input, etc.).
    shell.run(move |event, pixmap, window| {
        // We only care about redraw events in this minimal example.
        let ShellEvent::Redraw { scale_factor } = event else {
            return;
        };

        // --- Layout ---------------------------------------------------
        // Compute the layout in logical points, then scale to physical
        // pixels for the canvas.
        let logical_size = Size {
            width: window.inner_size().width as f32 / scale_factor,
            height: window.inner_size().height as f32 / scale_factor,
        };
        let layout = compute_layout(&arena, root, logical_size, scale_factor);

        // --- Draw -----------------------------------------------------
        // Walk the widget tree and call `draw` on each widget, passing
        // the canvas and the widget's computed rect (physical pixels).
        arena.traverse(root, |id, widget| {
            if let Some(rect) = layout.get(id) {
                widget.draw(pixmap, *rect);
            }
        });
    })?;

    Ok(())
}
