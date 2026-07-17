//! # Typography Gallery — Text Rendering Showcase
//!
//! Demonstrates the exact font-metric measurement and glyph rasterization
//! pipeline across multiple sizes.  All labels use a single `Column` with
//! a dark theme.  No interactivity — purely a visual verification tool.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example typography_gallery
//! ```

use lath::core::UiArena;
use lath::layout::{compute_layout, Size};
use lath::shell::{ShellEvent, WindowShell};
use lath::widgets::{Column, Container, Label};
use tiny_skia::Color;

enum Void {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arena = UiArena::<Void>::new();

    // -----------------------------------------------------------------------
    // Build the layout tree (bottom-up)
    // -----------------------------------------------------------------------

    // Title block — large bold-style text.
    let title = arena.spawn(
        Label::new("Typography Gallery")
            .text_color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF))
            .font_size(36.0),
    );

    // Longer paragraph.
    let paragraph = arena.spawn(
        Label::new(
            "The quick brown fox jumps over the lazy dog. \
             Pack my box with five dozen liquor jugs.",
        )
        .text_color(Color::from_rgba8(0xCC, 0xCC, 0xCC, 0xFF))
        .font_size(16.0),
    );

    // Sidecar: a bordered container wrapping a short label, placed next
    // to the paragraph in the column to verify that layout bounding
    // rects match text bounds exactly.
    let sidecar_label = arena.spawn(
        Label::new("Bounded Box")
            .text_color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF))
            .font_size(14.0),
    );
    let sidecar = arena.spawn(
        Container::new()
            .padding(12.0)
            .bg_color(Color::from_rgba8(0x35, 0x35, 0x35, 0xFF))
            .child(sidecar_label),
    );

    // Size ramp: from tiny captions up to display text.
    let sizes: &[(f32, &str)] = &[
        (10.0, "Caption — 10px"),
        (16.0, "Body — 16px"),
        (24.0, "Heading — 24px"),
        (36.0, "Title — 36px"),
        (48.0, "Display — 48px"),
    ];
    let mut ramp_ids = Vec::new();
    for &(size, text) in sizes {
        ramp_ids.push(
            arena.spawn(
                Label::new(text)
                    .text_color(Color::from_rgba8(0xEE, 0xEE, 0xEE, 0xFF))
                    .font_size(size),
            ),
        );
    }

    // Column stacks everything vertically.
    let mut col = Column::new()
        .spacing(16.0)
        .child(title)
        .child(paragraph)
        .child(sidecar);
    for &id in &ramp_ids {
        col = col.child(id);
    }
    let inner = arena.spawn(col);

    // Outer container wraps the column with a dark background.
    let root = arena.spawn(
        Container::new()
            .padding(24.0)
            .bg_color(Color::from_rgba8(0x1E, 0x1E, 0x1E, 0xFF))
            .child(inner),
    );
    arena.set_root(root);

    // -----------------------------------------------------------------------
    // Run the window
    // -----------------------------------------------------------------------

    let shell = WindowShell::new("lath — Typography Gallery", 680, 700)?;

    shell.run(move |event, pixmap, window| {
        let ShellEvent::Redraw { scale_factor } = event else {
            return;
        };

        let logical_size = Size {
            width: window.inner_size().width as f32 / scale_factor,
            height: window.inner_size().height as f32 / scale_factor,
        };
        let layout = compute_layout(&arena, root, logical_size, scale_factor);

        arena.traverse(root, |id, widget| {
            if let Some(rect) = layout.get(id) {
                widget.draw(pixmap, *rect);
            }
        });
    })?;

    Ok(())
}
