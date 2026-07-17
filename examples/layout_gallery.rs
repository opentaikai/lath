//! # Layout Gallery — Structural Layout Example
//!
//! Demonstrates `Column`, `Row`, `Container`, `Label`, and `Button`
//! working together in a nested layout hierarchy.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example layout_gallery
//! ```

use lath::core::UiArena;
use lath::layout::{compute_layout, Size};
use lath::shell::{ShellEvent, WindowShell};
use lath::widgets::{Button, Column, Container, Label, Row};
use tiny_skia::Color;

/// Application message type (unused in this static layout example).
#[derive(Clone, Default)]
enum Msg {
    #[default]
    None,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arena = UiArena::<Msg>::new();

    // -----------------------------------------------------------------------
    // Build the tree bottom-up (children first, then parents)
    // -----------------------------------------------------------------------

    // Leaf: status bar label
    let status_label = arena.spawn(
        Label::new("Ready")
            .text_color(Color::from_rgba8(0xCC, 0xCC, 0xCC, 0xFF))
            .font_size(14.0),
    );

    // Footer: wraps the status label
    let footer = arena.spawn(
        Container::new()
            .padding(12.0)
            .bg_color(Color::from_rgba8(0x25, 0x25, 0x25, 0xFF))
            .child(status_label),
    );

    // Leaf: "Cancel" button label
    let cancel_label = arena.spawn(
        Label::new("Cancel")
            .text_color(Color::from_rgba8(0x33, 0x33, 0x33, 0xFF))
            .font_size(14.0),
    );

    // Leaf: "Cancel" button
    let cancel_btn = arena.spawn(
        Button::<Msg>::new()
            .padding(10.0)
            .bg_color(Color::from_rgba8(0xE0, 0xE0, 0xE0, 0xFF))
            .child(cancel_label),
    );

    // Leaf: "Save" button label
    let save_label = arena.spawn(
        Label::new("Save")
            .text_color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF))
            .font_size(14.0),
    );

    // Leaf: "Save" button
    let save_btn = arena.spawn(
        Button::<Msg>::new()
            .padding(10.0)
            .bg_color(Color::from_rgba8(0x40, 0x80, 0xF0, 0xFF))
            .child(save_label),
    );

    // Row: Save + Cancel side by side
    let button_row = arena.spawn(Row::new().spacing(8.0).child(save_btn).child(cancel_btn));

    // Leaf: action panel description
    let action_label = arena.spawn(
        Label::new("Account Actions")
            .text_color(Color::from_rgba8(0xEE, 0xEE, 0xEE, 0xFF))
            .font_size(16.0),
    );

    // Column: action panel stacks the label + button row
    let action_panel = arena.spawn(
        Column::new()
            .spacing(16.0)
            .child(action_label)
            .child(button_row),
    );

    // Container: right action panel wrapper with background
    let right_panel = arena.spawn(
        Container::new()
            .padding(20.0)
            .bg_color(Color::from_rgba8(0x35, 0x35, 0x35, 0xFF))
            .child(action_panel),
    );

    // Leaf: metric info label
    let metric_label = arena.spawn(
        Label::new("Total Users: 1,234\nActive Sessions: 56\nUptime: 99.9%")
            .text_color(Color::from_rgba8(0xDD, 0xDD, 0xDD, 0xFF))
            .font_size(14.0),
    );

    // Container: left panel with background
    let left_panel = arena.spawn(
        Container::new()
            .padding(20.0)
            .bg_color(Color::from_rgba8(0x2D, 0x2D, 0x2D, 0xFF))
            .child(metric_label),
    );

    // Row: body split-view
    let body = arena.spawn(Row::new().spacing(12.0).child(left_panel).child(right_panel));

    // Leaf: header title
    let title_label = arena.spawn(
        Label::new("User Profile")
            .text_color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF))
            .font_size(22.0),
    );

    // Container: header bar
    let header = arena.spawn(
        Container::new()
            .padding(16.0)
            .bg_color(Color::from_rgba8(0x1E, 0x1E, 0x1E, 0xFF))
            .child(title_label),
    );

    // Root Column: stacks header, body, footer
    let root = arena.spawn(
        Column::new()
            .spacing(0.0)
            .child(header)
            .child(body)
            .child(footer),
    );

    arena.set_root(root);

    // -----------------------------------------------------------------------
    // Run the window
    // -----------------------------------------------------------------------

    let shell = WindowShell::new("lath — Layout Gallery", 600, 400)?;

    shell.run(move |event, pixmap, window| {
        let ShellEvent::Redraw { scale_factor } = event else { return };

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
