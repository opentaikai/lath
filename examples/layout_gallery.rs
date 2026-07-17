//! # Layout Gallery — Structural Layout Example
//!
//! Demonstrates all six built-in widgets (`Column`, `Row`, `Container`,
//! `Label`, `Button`) working together in a nested layout hierarchy.
//! Save / Cancel buttons print to stdout when clicked.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example layout_gallery
//! ```

use lath::events::{dispatch_event, UiContext};
use lath::layout::{compute_layout, Point, Size};
use lath::shell::{ShellEvent, WindowShell};
use lath::widgets::{Button, Column, Container, Label, Row};
use tiny_skia::Color;

/// Application messages emitted by interactive widgets.
#[derive(Clone, Default)]
enum Msg {
    #[default]
    None,
    Save,
    Cancel,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = UiContext::<Msg>::new();

    // -----------------------------------------------------------------------
    // Build the tree bottom-up (children first, then parents)
    // -----------------------------------------------------------------------

    // Leaf: status bar label
    let status_label = ui.arena.spawn(
        Label::new("Ready")
            .text_color(Color::from_rgba8(0xCC, 0xCC, 0xCC, 0xFF))
            .font_size(14.0),
    );

    // Footer: wraps the status label
    let footer = ui.arena.spawn(
        Container::new()
            .padding(12.0)
            .bg_color(Color::from_rgba8(0x25, 0x25, 0x25, 0xFF))
            .child(status_label),
    );

    // Leaf: "Cancel" button label
    let cancel_label = ui.arena.spawn(
        Label::new("Cancel")
            .text_color(Color::from_rgba8(0x33, 0x33, 0x33, 0xFF))
            .font_size(14.0),
    );

    // Leaf: "Cancel" button (emits Msg::Cancel on click)
    let cancel_btn = ui.arena.spawn(
        Button::new()
            .padding(10.0)
            .bg_color(Color::from_rgba8(0xE0, 0xE0, 0xE0, 0xFF))
            .child(cancel_label)
            .on_click(Msg::Cancel),
    );

    // Leaf: "Save" button label
    let save_label = ui.arena.spawn(
        Label::new("Save")
            .text_color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF))
            .font_size(14.0),
    );

    // Leaf: "Save" button (emits Msg::Save on click)
    let save_btn = ui.arena.spawn(
        Button::new()
            .padding(10.0)
            .bg_color(Color::from_rgba8(0x40, 0x80, 0xF0, 0xFF))
            .child(save_label)
            .on_click(Msg::Save),
    );

    // Row: Save + Cancel side by side
    let button_row = ui.arena
        .spawn(Row::new().spacing(8.0).child(save_btn).child(cancel_btn));

    // Leaf: action panel description
    let action_label = ui.arena.spawn(
        Label::new("Account Actions")
            .text_color(Color::from_rgba8(0xEE, 0xEE, 0xEE, 0xFF))
            .font_size(16.0),
    );

    // Column: action panel stacks the label + button row
    let action_panel = ui.arena.spawn(
        Column::new()
            .spacing(16.0)
            .child(action_label)
            .child(button_row),
    );

    // Container: right action panel wrapper with background
    let right_panel = ui.arena.spawn(
        Container::new()
            .padding(20.0)
            .bg_color(Color::from_rgba8(0x35, 0x35, 0x35, 0xFF))
            .child(action_panel),
    );

    // Leaf: metric info label
    let metric_label = ui.arena.spawn(
        Label::new("Total Users: 1,234\nActive Sessions: 56\nUptime: 99.9%")
            .text_color(Color::from_rgba8(0xDD, 0xDD, 0xDD, 0xFF))
            .font_size(14.0),
    );

    // Container: left panel with background
    let left_panel = ui.arena.spawn(
        Container::new()
            .padding(20.0)
            .bg_color(Color::from_rgba8(0x2D, 0x2D, 0x2D, 0xFF))
            .child(metric_label),
    );

    // Row: body split-view
    let body = ui.arena.spawn(
        Row::new().spacing(12.0).child(left_panel).child(right_panel),
    );

    // Leaf: header title
    let title_label = ui.arena.spawn(
        Label::new("User Profile")
            .text_color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF))
            .font_size(22.0),
    );

    // Container: header bar
    let header = ui.arena.spawn(
        Container::new()
            .padding(16.0)
            .bg_color(Color::from_rgba8(0x1E, 0x1E, 0x1E, 0xFF))
            .child(title_label),
    );

    // Root Column: stacks header, body, footer
    let root = ui.arena.spawn(
        Column::new()
            .spacing(0.0)
            .child(header)
            .child(body)
            .child(footer),
    );

    ui.arena.set_root(root);

    // -----------------------------------------------------------------------
    // Run the window
    // -----------------------------------------------------------------------

    let shell = WindowShell::new("lath — Layout Gallery", 600, 400)?;

    shell.run(move |event, pixmap, window| {
        // 1. Drain any button-click messages.
        for msg in ui.drain_events() {
            match msg {
                Msg::Save => println!("Save clicked"),
                Msg::Cancel => println!("Cancel clicked"),
                Msg::None => {}
            }
        }

        let Some(root) = ui.arena.root() else {
            return;
        };

        // 2. Compute layout for the current event.
        let scale_factor = match event {
            ShellEvent::Redraw { scale_factor }
            | ShellEvent::Resized {
                scale_factor, ..
            } => scale_factor,
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
