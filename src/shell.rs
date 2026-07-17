use std::num::NonZeroU32;
use std::sync::Arc;

use softbuffer::{Context, Surface};
use tiny_skia::PixmapMut;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton as WinitMouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Platform-agnostic window event translated from winit system events.
///
/// All coordinate values in [`ShellEvent`] are in **physical pixels**.
#[derive(Debug, Clone, Copy)]
pub enum ShellEvent {
    /// The window surface should be redrawn.  The closure receives the
    /// current `scale_factor` so the caller can compute layout in logical
    /// points before drawing onto the physical-pixel canvas.
    Redraw { scale_factor: f32 },
    /// The cursor moved to a new position (physical pixels).
    CursorMoved { x: f64, y: f64 },
    /// A mouse button was pressed at the given position (physical pixels).
    MouseButtonPressed { x: f64, y: f64, button: MouseButton },
    /// A mouse button was released at the given position (physical pixels).
    MouseButtonReleased { x: f64, y: f64, button: MouseButton },
    /// The window was resized (physical pixel dimensions) or the display
    /// scale factor changed.  The `scale_factor` field lets the caller
    /// derive logical size as `physical / scale_factor`.
    Resized {
        width: u32,
        height: u32,
        scale_factor: f32,
    },
    /// The display scale factor changed (e.g. window moved to a different
    /// monitor).  The new `scale_factor` is provided for cache invalidation.
    ScaleFactorChanged { scale_factor: f32 },
}

/// Abstracts the platform window, pixel buffer context, and event loop.
///
/// Creates the OS window and manages the softbuffer rendering surface.
/// All widget or layout logic belongs in higher-level modules.
pub struct WindowShell {
    title: String,
    width: u32,
    height: u32,
}

impl WindowShell {
    /// Creates a new window shell with the given title and dimensions.
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            title: title.to_string(),
            width,
            height,
        })
    }

    /// Runs the event loop, dispatching events to the provided closure.
    ///
    /// The closure receives a [`ShellEvent`], a mutable reference to the
    /// pixel canvas ([`PixmapMut`]), and a reference to the underlying
    /// [`winit::window::Window`].
    pub fn run<F>(self, event_handler: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(ShellEvent, &mut PixmapMut, &Window),
    {
        let event_loop = EventLoop::new()?;

        let mut app = ShellApp {
            title: self.title,
            width: self.width,
            height: self.height,
            window: None,
            context: None,
            surface: None,
            rgba_buffer: Vec::new(),
            cursor_position: (0.0, 0.0),
            scale_factor: 1.0,
            event_handler,
        };

        event_loop.run_app(&mut app)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Internal application state
// ---------------------------------------------------------------------------

struct ShellApp<F> {
    title: String,
    width: u32,
    height: u32,
    window: Option<Arc<Window>>,
    context: Option<Context<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    rgba_buffer: Vec<u8>,
    cursor_position: (f64, f64),
    scale_factor: f32,
    event_handler: F,
}

impl<F> ShellApp<F> {
    /// Wraps the RGBA buffer in a [`PixmapMut`] and invokes the user closure.
    fn dispatch(
        handler: &mut F,
        event: ShellEvent,
        rgba_buffer: &mut [u8],
        width: u32,
        height: u32,
        window: &Window,
    ) where
        F: FnMut(ShellEvent, &mut PixmapMut, &Window),
    {
        let mut pixmap = PixmapMut::from_bytes(rgba_buffer, width, height)
            .expect("RGBA buffer has valid dimensions");
        handler(event, &mut pixmap, window);
    }

    /// Converts the RGBA pixel buffer to softbuffer's native u32 format
    /// and presents the frame to the display.
    fn present_frame(surface: &mut Surface<Arc<Window>, Arc<Window>>, rgba_buffer: &[u8]) {
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };

        for (dst, src) in buffer.iter_mut().zip(rgba_buffer.chunks_exact(4)) {
            let r = src[0] as u32;
            let g = src[1] as u32;
            let b = src[2] as u32;
            // softbuffer native format on Linux: 0x00RRGGBB
            *dst = (r << 16) | (g << 8) | b;
        }

        let _ = buffer.present();
    }
}

// ---------------------------------------------------------------------------
// winit ApplicationHandler implementation
// ---------------------------------------------------------------------------

impl<F> ApplicationHandler for ShellApp<F>
where
    F: FnMut(ShellEvent, &mut PixmapMut, &Window),
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // On platforms that deliver multiple `resumed` events, only create
        // the window once.
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(PhysicalSize::new(self.width, self.height));

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("failed to create window"),
        );

        // Query the initial scale factor from the display.
        self.scale_factor = window.scale_factor() as f32;

        let context = Context::new(window.clone()).expect("failed to create softbuffer context");
        let mut surface =
            Surface::new(&context, window.clone()).expect("failed to create softbuffer surface");
        surface
            .resize(
                NonZeroU32::new(self.width).expect("window width must be > 0"),
                NonZeroU32::new(self.height).expect("window height must be > 0"),
            )
            .expect("failed to resize softbuffer surface");

        self.rgba_buffer
            .resize((self.width * self.height * 4) as usize, 0);

        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        match event {
            // ---- lifecycle ------------------------------------------------
            WindowEvent::CloseRequested => event_loop.exit(),

            // ---- resize ---------------------------------------------------
            WindowEvent::Resized(size) => {
                let old_scale = self.scale_factor;
                self.scale_factor = window.scale_factor() as f32;
                self.width = size.width;
                self.height = size.height;

                if let Some(surface) = &mut self.surface {
                    let _ = surface.resize(
                        NonZeroU32::new(self.width).unwrap(),
                        NonZeroU32::new(self.height).unwrap(),
                    );
                }

                self.rgba_buffer
                    .resize((self.width * self.height * 4) as usize, 0);

                // If the scale factor changed (e.g. window moved to a
                // different monitor), emit a dedicated event so callers
                // can invalidate caches.
                if (self.scale_factor - old_scale).abs() > f32::EPSILON {
                    Self::dispatch(
                        &mut self.event_handler,
                        ShellEvent::ScaleFactorChanged {
                            scale_factor: self.scale_factor,
                        },
                        &mut self.rgba_buffer,
                        self.width,
                        self.height,
                        window,
                    );
                }

                Self::dispatch(
                    &mut self.event_handler,
                    ShellEvent::Resized {
                        width: self.width,
                        height: self.height,
                        scale_factor: self.scale_factor,
                    },
                    &mut self.rgba_buffer,
                    self.width,
                    self.height,
                    window,
                );

                window.request_redraw();
            }

            // ---- redraw ---------------------------------------------------
            WindowEvent::RedrawRequested => {
                let byte_count = (self.width * self.height * 4) as usize;
                self.rgba_buffer.resize(byte_count, 0);

                // Clear the frame to wheat (#F5DEB3).
                for pixel in self.rgba_buffer.chunks_exact_mut(4) {
                    pixel[0] = 0xF5; // R
                    pixel[1] = 0xDE; // G
                    pixel[2] = 0xB3; // B
                    pixel[3] = 0xFF; // A
                }

                // Let the user draw on the canvas, passing the current
                // scale factor so layout can be computed in logical points.
                Self::dispatch(
                    &mut self.event_handler,
                    ShellEvent::Redraw {
                        scale_factor: self.scale_factor,
                    },
                    &mut self.rgba_buffer,
                    self.width,
                    self.height,
                    window,
                );

                // Convert RGBA → softbuffer and present.
                if let Some(surface) = &mut self.surface {
                    Self::present_frame(surface, &self.rgba_buffer);
                }
            }

            // ---- input ----------------------------------------------------
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = (position.x, position.y);

                Self::dispatch(
                    &mut self.event_handler,
                    ShellEvent::CursorMoved {
                        x: position.x,
                        y: position.y,
                    },
                    &mut self.rgba_buffer,
                    self.width,
                    self.height,
                    window,
                );
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let shell_button = match button {
                    WinitMouseButton::Left => MouseButton::Left,
                    WinitMouseButton::Right => MouseButton::Right,
                    WinitMouseButton::Middle => MouseButton::Middle,
                    _ => return,
                };

                let shell_event = match state {
                    ElementState::Pressed => ShellEvent::MouseButtonPressed {
                        x: self.cursor_position.0,
                        y: self.cursor_position.1,
                        button: shell_button,
                    },
                    ElementState::Released => ShellEvent::MouseButtonReleased {
                        x: self.cursor_position.0,
                        y: self.cursor_position.1,
                        button: shell_button,
                    },
                };

                Self::dispatch(
                    &mut self.event_handler,
                    shell_event,
                    &mut self.rgba_buffer,
                    self.width,
                    self.height,
                    window,
                );

                // Request a redraw so the user closure can react to input
                // (e.g. update counter state and re-render).
                window.request_redraw();
            }

            _ => {}
        }
    }
}
