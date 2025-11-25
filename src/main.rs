use anyhow::Result;
use vulkanalia::vk::DeviceV1_0;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

use crate::app::App;

pub mod app;
pub mod foundation;
pub mod pipeline;
pub mod scenes;

fn main() -> Result<()> {
    pretty_env_logger::init();

    // window creation

    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("CHOAM (VK)")
        .with_inner_size(LogicalSize::new(1024, 768))
        .with_visible(true)
        .build(&event_loop)?;

    // app creation

    let mut app = unsafe { App::create(&window)? };
    let mut minimized = false;
    event_loop.run(move |event, elwt| {
        match event {
            // request redraw when all events are processed
            Event::AboutToWait => window.request_redraw(),
            Event::WindowEvent { event, .. } => match event {
                // render a frame if the vulkna app is not being destroyed
                WindowEvent::RedrawRequested if !elwt.exiting() && !minimized => {
                    unsafe { app.render(&window) }.unwrap()
                }
                // destroy the vulkan app
                WindowEvent::CloseRequested => {
                    elwt.exit();
                    unsafe {
                        app.device.device_wait_idle().unwrap();
                        app.destroy();
                    }
                }
                WindowEvent::Resized(size) => {
                    if size.width == 0 || size.height == 0 {
                        minimized = true;
                    } else {
                        minimized = false;
                        app.resized = true;
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == ElementState::Pressed {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::ArrowLeft) if app.models > 1 => {
                                app.models -= 1
                            }
                            PhysicalKey::Code(KeyCode::ArrowRight) if app.models < 4 => {
                                app.models += 1
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    })?;

    println!("Hello, world!");
    Ok(())
}
