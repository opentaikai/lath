use lath::shell::{ShellEvent, WindowShell};
use tiny_skia::{Color, Paint, PixmapMut, Rect, Transform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shell = WindowShell::new("Lath", 800, 600)?;

    shell.run(|event, pixmap, _window| match event {
        ShellEvent::Redraw { .. } => {
            draw_centered_box(pixmap);
        }
        ShellEvent::CursorMoved { x, y } => {
            println!("cursor ({x:.0}, {y:.0})");
        }
        ShellEvent::MouseButtonPressed { x, y, button } => {
            println!("pressed ({x:.0}, {y:.0}) {button:?}");
        }
        ShellEvent::MouseButtonReleased { x, y, button } => {
            println!("released ({x:.0}, {y:.0}) {button:?}");
        }
        ShellEvent::Resized {
            width,
            height,
            ..
        } => {
            println!("resized {width}x{height}");
        }
        ShellEvent::ScaleFactorChanged { scale_factor } => {
            println!("scale factor changed to {scale_factor}");
        }
    })?;

    Ok(())
}

fn draw_centered_box(pixmap: &mut PixmapMut) {
    let rect = Rect::from_xywh(250.0, 175.0, 300.0, 250.0).unwrap();
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(0x50, 0x50, 0xFA, 0xFF));
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);

    let inner = Rect::from_xywh(270.0, 195.0, 260.0, 210.0).unwrap();
    let mut paint2 = Paint::default();
    paint2.set_color(Color::from_rgba8(0xFA, 0x50, 0x50, 0xFF));
    pixmap.fill_rect(inner, &paint2, Transform::identity(), None);
}
