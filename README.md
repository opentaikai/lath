# lath

A native, retained-mode GUI library for Rust, built from scratch.

`lath` provides a simple, flat-storage widget tree with a pure, two-pass layout solver and an Elm-style message queue — no `Rc<RefCell<T>>`, no smart-pointer cycles, no complex macro magic.

## Features

- **Arena Tree** — Widgets live in a flat `Vec`, referenced by lightweight copyable `WidgetId` handles.
- **Two-Pass Layout** — Bottom-up measure, top-down arrange. Pure, deterministic, easy to debug.
- **Event Dispatch** — Hit-testing maps cursor coordinates to widget bounds; interactive widgets emit messages through an `mpsc` channel.
- **Concrete Primitives** — `Container`, `Label`, and `Button` out of the box.
- **Platform Shell** — Windowing via `winit`, pixel buffer via `softbuffer`, 2D drawing via `tiny-skia`.

## Quick Start

```sh
# Clone and run the Hello World example
git clone https://github.com/opentaikai/lath.git
cd lath
cargo run --example hello_world
```

## Examples

### Hello World

A minimal example showing window creation, widget tree construction, and the render loop:

```rust
use lath::core::UiArena;
use lath::layout::{compute_layout, Size};
use lath::shell::{ShellEvent, WindowShell};
use lath::widgets::{Container, Label};
use tiny_skia::Color;

enum Void {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arena = UiArena::<Void>::new();

    let label = arena.spawn(
        Label::new("Hello, World from Lath!")
            .text_color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF))
            .font_size(28.0),
    );

    let root = arena.spawn(
        Container::new()
            .padding(40.0)
            .bg_color(Color::from_rgba8(0x1E, 0x1E, 0x1E, 0xFF))
            .child(label),
    );

    arena.set_root(root);

    let shell = WindowShell::new("lath — Hello World", 800, 600)?;

    shell.run(move |event, pixmap, window| {
        let ShellEvent::Redraw = event else { return };

        let window_size = Size {
            width: window.inner_size().width as f32,
            height: window.inner_size().height as f32,
        };
        let layout = compute_layout(&arena, root, window_size);

        arena.traverse(root, |id, widget| {
            if let Some(rect) = layout.get(id) {
                widget.draw(pixmap, *rect);
            }
        });
    })?;

    Ok(())
}
```

Run it:

```sh
cargo run --example hello_world
```

## Architecture

```
src/
├── shell.rs          Platform windowing (winit + softbuffer + tiny-skia)
├── core.rs           Arena tree: WidgetId, UiArena<M>
├── widget.rs         Widget<M> trait: measure, arrange, draw, handle_event
├── layout.rs         Two-pass layout solver: Constraints, LayoutState, compute_layout
├── events.rs         Hit-testing, EventCtx<M>, UiContext<M>, dispatch_event
├── widgets/
│   ├── mod.rs        Public re-exports
│   ├── container.rs  Structural wrapper with padding + background
│   ├── label.rs      Static text leaf
│   └── button.rs     Interactive container with click message
└── lib.rs            Crate root
```

### Module Overview

| Module | Purpose |
|--------|---------|
| `shell` | Creates the OS window, manages the pixel buffer, and translates platform events into `ShellEvent` |
| `core` | Flat widget storage (`UiArena`) with copyable `WidgetId` handles — no pointers, no lifetimes |
| `widget` | The `Widget<M>` trait that every widget implements: layout hooks, drawing, and event handling |
| `layout` | Pure mathematical layout engine: `Constraints`, `Size`, `Rect`, `compute_layout()` |
| `events` | Top-down hit-testing, `EventCtx` for emitting messages, `UiContext` that owns the arena + channel |
| `widgets` | Concrete primitives: `Container`, `Label`, `Button` |

### The Layout Model

`lath` enforces a strict two-pass layout:

1. **Measure (bottom-up)** — Parents pass `Constraints` down; children return their preferred `Size`.
2. **Arrange (top-down)** — Parents assign concrete `(x, y)` offsets to children; the solver records final `Rect` bounds.

```rust
let layout = compute_layout(&arena, root, window_size);
let rect = layout.get(widget_id); // Option<&Rect>
```

### Event Dispatch

Widgets do not use closure callbacks. Instead:

1. The `Button` stores a message `M` at construction time.
2. On click, `dispatch_event` hit-tests the widget tree and calls `handle_event(Click, ctx)`.
3. The button clones its stored message and pushes it into an `mpsc` channel.
4. Your main loop drains the channel with `ui.drain_events()`.

```rust
#[derive(Clone)]
enum AppMsg {
    ButtonClicked,
}

let mut ui = UiContext::<AppMsg>::new();
let btn = ui.arena.spawn(
    Button::new()
        .child(label)
        .on_click(AppMsg::ButtonClicked),
);

// In your event loop:
for msg in ui.drain_events() {
    match msg {
        AppMsg::ButtonClicked => { /* update state */ }
    }
}
```

## Dependencies

| Crate | Version | Role |
|-------|---------|------|
| `winit` | 0.30 | Cross-platform windowing |
| `softbuffer` | 0.4 | CPU pixel buffer |
| `tiny-skia` | 0.12 | 2D vector rendering |
| `bytemuck` | 1.25 | Safe transmute for pixel data |

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.
