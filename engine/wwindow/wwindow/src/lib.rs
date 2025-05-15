use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct State<'window> {
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
}

impl<'window> State<'window> {
    pub async fn new(window: Window) -> State<'window> {
        // Configure instance based on platform
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let size = winit::dpi::PhysicalSize::new(800, 600);
                let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                    backends: wgpu::Backends::GL,
                    ..Default::default()
                });
                let limits = wgpu::Limits::downlevel_webgl2_defaults();
            } else {
                let size = window.inner_size();
                let instance = wgpu::Instance::default();
                let limits = wgpu::Limits::default();
            }
        }

        // Create surface using the same method as the working example
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap())
        }
        .expect("Failed to create surface");

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find a suitable GPU adapter");

        log::info!("Using GPU: {}", adapter.get_info().name);

        // Create device with proper limits
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                },
                None,
            )
            .await
            .expect("Failed to create device");

        // Configure surface
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();

        surface.configure(&device, &config);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.8,
                            b: 0.2,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    log::info!("Starting multiplatform window application");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Multiplatform Window")
        .build(&event_loop)
        .unwrap();

    // Set up canvas for web target - using the exact same pattern as the working code
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let canvas = web_sys::Element::from(window.canvas().unwrap());
                // Try to get element by id "app" first, then fall back to body
                doc.get_element_by_id("app")
                    .or_else(|| doc.body().map(|body| body.into()))
                    .and_then(|element| {
                        element.append_child(&canvas).ok()
                    })
            })
            .expect("Couldn't append canvas to document");
    }

    run_app(event_loop, window).await;
}

async fn run_app(event_loop: EventLoop<()>, window: Window) {
    let mut state = State::new(window).await;

    event_loop
        .run(move |event, control_flow| match event {
            Event::WindowEvent {
                event,
                window_id,
            } if window_id == state.window().id() => match event {
                WindowEvent::CloseRequested => control_flow.exit(),
                WindowEvent::Resized(physical_size) => {
                    state.resize(physical_size);
                }
                WindowEvent::RedrawRequested => {
                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.window().inner_size()),
                        Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                        Err(e) => log::error!("render error: {e:?}"),
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                state.window().request_redraw();
            }
            _ => {}
        })
        .expect("event loop runs");
}