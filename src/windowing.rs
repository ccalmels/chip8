use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

pub trait Renderable {
    const WIDTH: u32;
    const HEIGHT: u32;

    fn render(&self, pixels: &mut Pixels<'_>);
}

pub trait Updatable {
    type Error;
    fn update(&mut self) -> Result<(), Self::Error>;
}

pub struct App<E: Renderable + Updatable> {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    engine: E,
    pub engine_error: Option<E::Error>,
}

impl<E: Renderable + Updatable> App<E> {
    pub fn new(engine: E) -> Self {
        Self {
            window: None,
            pixels: None,
            engine,
            engine_error: None,
        }
    }

    pub fn run(&mut self) {
        let event_loop = EventLoop::new().unwrap();

        event_loop.set_control_flow(ControlFlow::Wait);

        let _ = event_loop.run_app(self);
    }
}

impl<E: Renderable + Updatable> ApplicationHandler for App<E> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Chip-8")
                        .with_inner_size(LogicalSize::new(E::WIDTH * 12, E::HEIGHT * 12)),
                )
                .unwrap();

            window.set_cursor_visible(false);
            let window_size = window.inner_size();

            let window = Arc::new(window);
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, Arc::clone(&window));
            let pixels = Pixels::new(E::WIDTH, E::HEIGHT, surface_texture).unwrap();

            self.window = Some(window);
            self.pixels = Some(pixels);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let (Some(pixels), Some(window)) = (&mut self.pixels, &self.window) {
                    let rc = self.engine.update();

                    if let Err(error) = rc {
                        self.engine_error = Some(error);
                        event_loop.exit();
                        return;
                    }

                    self.engine.render(pixels);

                    pixels.render().unwrap();
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}
