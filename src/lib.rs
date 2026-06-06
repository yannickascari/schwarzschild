pub mod renderer;
pub mod camera;

use std::sync::Arc;
use winit::event::{ElementState, MouseScrollDelta};
use renderer::GPUState;
use crate::camera::OrbitalCamera;
use winit::{
    application::ApplicationHandler,
    event::{WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GPUState>,
    camera: OrbitalCamera,
    mouse_pressed: bool,
    last_mouse: Option<(f32, f32)>,
    #[cfg(target_arch = "wasm32")]
    gpu_pending: std::rc::Rc<std::cell::RefCell<Option<GPUState>>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop.create_window(
                {
                    let attrs = Window::default_attributes().with_title("Schwarzschild");
                    #[cfg(not(target_arch = "wasm32"))]
                    let attrs = attrs.with_maximized(true);
                    attrs
                }
            ).unwrap()
        );

        #[cfg(target_arch = "wasm32")]
        {
            let canvas = window.canvas().unwrap();
            web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.body())
                .and_then(|b| b.append_child(&canvas).ok());
        }

        self.window = Some(window.clone());

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.gpu = Some(pollster::block_on(GPUState::new(window)));
        }

        #[cfg(target_arch = "wasm32")]
        {
            let pending = self.gpu_pending.clone();
            let window_ref = window.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let gpu = GPUState::new(window).await;
                *pending.borrow_mut() = Some(gpu);
                window_ref.request_redraw();
            });
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(new_size);
                }
            }
            WindowEvent::RedrawRequested => {
                #[cfg(target_arch = "wasm32")]
                if self.gpu.is_none() {
                    if self.gpu_pending.borrow().is_some() {
                        self.gpu = self.gpu_pending.borrow_mut().take();
                    }
                }

                if let Some(gpu) = &mut self.gpu {
                    let uniform = self.camera.to_uniform();
                    gpu.update_camera(&uniform);
                    gpu.render();
                }

                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Left {
                    self.mouse_pressed = state == ElementState::Pressed;
                    if !self.mouse_pressed {
                        self.last_mouse = None;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = (position.x as f32, position.y as f32);
                if self.mouse_pressed {
                    if let Some((lx, ly)) = self.last_mouse {
                        let dx = (pos.0 - lx) * 0.01;
                        let dy = (pos.1 - ly) * 0.01;
                        self.camera.orbit(dx, dy);
                    }
                }
                self.last_mouse = Some(pos);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.01,
                };
                self.camera.zoom(-scroll);
            }
            _ => {}
        }
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

pub fn run() {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Warn).ok();
    }

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        window: None,
        gpu: None,
        camera: OrbitalCamera::new(),
        mouse_pressed: false,
        last_mouse: None,
        #[cfg(target_arch = "wasm32")]
        gpu_pending: std::rc::Rc::new(std::cell::RefCell::new(None)),
    };
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_start() {
    run();
}