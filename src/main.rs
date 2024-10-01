use ash::vk::PhysicalDevice;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::WindowAttributes
};
use std::{
    default::Default, error::Error
};

mod graphics;

fn create_window(title: &str, width: u32, height: u32, event_loop: &winit::event_loop::ActiveEventLoop) -> Result<winit::window::Window, winit::error::OsError> {
    let window_attributes = WindowAttributes::default()
        .with_title(title)
        .with_inner_size(winit::dpi::LogicalSize::new(
            f64::from(width),
            f64::from(height),
        ))
        .with_resizable(false);

    event_loop.create_window(window_attributes)
}

enum WinitApp {
    None,
    Resumed(Application),
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        *self = Self::Resumed(Application::new(event_loop).unwrap());
    }

    fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _: winit::window::WindowId,
            event: WindowEvent) 
    {
        if let Self::Resumed(app) = self {
            app.window_event(event_loop, event);
        }
    }
}

struct Application {
    window: winit::window::Window,
    device: graphics::Device,
}

impl Application {
    pub fn new(event_loop: &ActiveEventLoop) -> Result<Self, Box<dyn Error>> {
        let window = create_window("Lorr", 1280, 720, event_loop)?;
        unsafe {
            let device = graphics::Device::new(&window)?;
            Ok(Self {
                window,
                device
            })
        }
    }

    fn window_event(&self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    Ok(EventLoop::new()?.run_app(&mut WinitApp::None)?)
}

