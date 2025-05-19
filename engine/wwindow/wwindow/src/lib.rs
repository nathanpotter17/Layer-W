use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    #[cfg(not(target_arch = "wasm32"))]
    webview: Option<wry::WebView>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> State {
        // Configure instance based on platform
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let size = winit::dpi::PhysicalSize::new(1080, 720);
                let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                    backends: wgpu::Backends::BROWSER_WEBGPU,
                    ..Default::default()
                });
                let limits = wgpu::Limits::downlevel_webgl2_defaults();
            } else {
                let size = window.inner_size();
                let instance = wgpu::Instance::default();
                let limits = wgpu::Limits::default();

                // Setup familiar web context on native. Meant to interface with Walloc module for runtime asset streaming.
                #[cfg(not(target_arch = "wasm32"))]
                let webview = {
                    use wry::WebViewBuilder;
                    use winit::dpi::{LogicalPosition, LogicalSize};
                    
                    let html_content = r#"
                    <!DOCTYPE html>
                    <html>
                    <body>
                        <script>
                            window.sendMessage = function(message) {
                                // Use the proper Wry IPC mechanism
                                window.chrome.webview.postMessage(message);
                            };

                             setTimeout(() => {
                                window.sendMessage('WebView initialized');
                            }, 500);
                            
                            window.runJavaScript = function(code) {
                                console.log('Executing JS from Rust:', code);
                                try {
                                    let result = eval(code);
                                    return result;
                                } catch (e) {
                                    console.error('JS execution error:', e);
                                    return null;
                                }
                            };
                        </script>
                    </body>
                    </html>
                    "#;
                    
                    let data_url = format!("data:text/html,{}", urlencoding::encode(html_content));
                    
                    println!("Creating WebView as child window...");
                    
                    let webview = WebViewBuilder::new()
                        .with_url(&data_url)
                        .with_initialization_script(
                            r#"
                            if (window.chrome && window.chrome.webview) {
                                console.log('WebView2 API is available');
                            } else {
                                console.error('WebView2 API is not available');
                            }
                            "#
                        )
                        .with_visible(false)
                        .with_focused(false)
                        .with_transparent(true)
                        .with_clipboard(false)
                        // .with_ipc_handler(|message| { // Can implement capability based IPC like Tauri does
                        //     println!("IPC Message: {:?}", message);
                        // })
                        .build_as_child(&window);
                        
                    
                    match webview {
                        Ok(webview) => {
                            println!("Child WebView created successfully!");
                            Some(webview)
                        }
                        Err(e) => {
                            println!("Failed to create child WebView: {:?}", e);
                            None
                        }
                    }
                };
            }
        }

        // Create surface using the instance
        let surface = instance.create_surface(window.clone()).expect("Failed to create surface");

       // Request the Best GPU adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an adapter");

        let adapter_info = adapter.get_info();
        
        let adapter = if adapter_info.device_type == wgpu::DeviceType::IntegratedGpu {
            // Try again with a preference override if we found an integrated GPU
            if let Ok(discrete_adapter) = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                })
                .await
            {
                let discrete_info = discrete_adapter.get_info();
                if discrete_info.device_type == wgpu::DeviceType::DiscreteGpu {
                    println!("Found better discrete GPU: {} ({:?})", discrete_info.name, discrete_info.device_type);

                    discrete_adapter
                } else {
                    adapter
                }
            } else {
                adapter
            }
        } else {
            adapter
        };

        // Get final adapter information
        let adapter_info = adapter.get_info();

        #[cfg(not(target_arch = "wasm32"))]
        println!("Selected GPU: {} ({:?})", adapter_info.name, adapter_info.device_type);

        #[cfg(target_arch = "wasm32")]
        {
            // Log the selected on web by looking through the navigator.
            use wasm_bindgen::prelude::*;
    
            let js_code = r#"
                if (navigator.gpu) {
                    console.log("WebGPU is supported");
                    console.log("Hardware concurrency: " + navigator.hardwareConcurrency);
                    
                    console.log("Attempting to enumerate all available GPUs:");
                    
                    navigator.gpu.requestAdapter().then((adapter) => {
                        if (adapter) {
                            console.log(`Adapter: ${adapter.info.vendor} ${adapter.info.architecture} ${adapter.info.device} (${adapter.info.description})`);
                        }
                    });
                } else {
                    console.log("WebGPU is not supported");
                }
            "#;
            
            if let Some(window) = web_sys::window() {
                let eval_fn = js_sys::Function::new_with_args("code", "return eval(code)");
                let _ = eval_fn.call1(&JsValue::NULL, &JsValue::from_str(js_code));
            }
        }

        // Create device with proper limits
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: Default::default(),
                }
            )
            .await
            .expect("Failed to create device");

        // Configure surface
        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps.formats[0];
        
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![surface_format.add_srgb_suffix()],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            #[cfg(not(target_arch = "wasm32"))]
            webview,
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn execute_js(&self, js_code: &str) -> Result<(), String> {
        if let Some(webview) = &self.webview {
            webview.evaluate_script(js_code)
                .map_err(|e| format!("Failed to execute JavaScript: {:?}", e))
        } else {
            Err("WebView not initialized".to_string())
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.config.format.add_srgb_suffix()),
                ..Default::default()
            });

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
        self.window.pre_present_notify();
        output.present();

        Ok(())
    }
}

// A simple wrapper to handle state initialization in WebAssembly
#[cfg(target_arch = "wasm32")]
struct StateInitializer {
    window: Arc<Window>,
    app_ptr: *mut App,
}

#[cfg(target_arch = "wasm32")]
impl StateInitializer {
    fn new(window: Arc<Window>, app: &mut App) -> Self {
        StateInitializer {
            window,
            app_ptr: app as *mut App,
        }
    }

    async fn initialize(self) {
        web_sys::console::log_1(&"Starting state initialization...".into());
        
        // Create the state
        let state = State::new(self.window.clone()).await;
        
        web_sys::console::log_1(&"State initialized, updating App...".into());
        
        // Safety: We know this pointer is valid because the App lives longer than this async task
        unsafe {
            let app = &mut *self.app_ptr;
            app.state = Some(state);
            app.state_initializing = false;
            
            web_sys::console::log_1(&"App state updated!".into());
        }
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
    window: Option<Arc<Window>>,
    #[cfg(target_arch = "wasm32")]
    state_initializing: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window based on platform
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Multiplatform Window")
                )
                .unwrap(),
        );
        
        // Set up canvas for web target - Browser Only
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            web_sys::console::log_1(&"Setting up web canvas".into());
            
            if let Some(canvas) = window.canvas() {
                let web_window = web_sys::window().unwrap();
                let document = web_window.document().unwrap();
                
                // Try to get element by id "app" first, then fall back to body
                let container = document.get_element_by_id("app")
                    .unwrap_or_else(|| document.body().unwrap().into());
                
                container.append_child(&web_sys::Element::from(canvas))
                    .expect("Couldn't append canvas to document");
                
                web_sys::console::log_1(&"Canvas attached to document".into());
            }
            
            // Store window reference
            self.window = Some(window.clone());
            self.state_initializing = true;
            
            // Begin async initialization
            let initializer = StateInitializer::new(window.clone(), self);
            wasm_bindgen_futures::spawn_local(initializer.initialize());
            
            window.request_redraw();
            return;
        }
        
        // Native platform initialization (using pollster is safe for native)
        #[cfg(not(target_arch = "wasm32"))]
        {
            let state = pollster::block_on(State::new(window.clone()));
            self.state = Some(state);
            self.window = Some(window.clone());

            // Initialize webview on native platforms
            if let Some(state) = &self.state {
                std::thread::sleep(std::time::Duration::from_millis(1000));

                match state.execute_js("console.log('Rust JS evaluation working'); 'Success'") {
                    Ok(_) => println!("Basic JavaScript executed successfully"),
                    Err(e) => println!("Error executing JavaScript: {:?}", e),
                }
                
                // Wait for WebView2 to be fully initialized
                std::thread::sleep(std::time::Duration::from_millis(500));
                
                match state.execute_js("window.sendMessage('Direct message test');") {
                    Ok(_) => println!("Direct IPC message sent"),
                    Err(e) => println!("Error sending direct IPC message: {:?}", e),
                }
            }

            window.request_redraw();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Request redraw to keep animation going even during async initialization
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        #[cfg(target_arch = "wasm32")]
        {
            // Check if we have a window reference to handle events with
            let window = match &self.window {
                Some(window) => window,
                None => return,
            };
            
            if window.id() != id {
                return;
            }
            
            match event {
                WindowEvent::CloseRequested => {
                    web_sys::console::log_1(&"Close requested".into());
                    event_loop.exit();
                },
                WindowEvent::RedrawRequested => {
                    // If state is initialized, render
                    if let Some(state) = &mut self.state {
                        match state.render() {
                            Ok(_) => {
                                // do nothing for now
                            },
                            Err(wgpu::SurfaceError::Lost) => {
                                web_sys::console::warn_1(&"Surface lost, reconfiguring...".into());
                                state.resize(state.window().inner_size());
                            },
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                web_sys::console::error_1(&"Out of memory, exiting".into());
                                event_loop.exit();
                            },
                            Err(e) => {
                                web_sys::console::error_1(&format!("Render error: {:?}", e).into());
                            },
                        }
                    } else if self.state_initializing {
                        // If state is still initializing, just log and keep going
                        web_sys::console::log_1(&"State still initializing, skipping render".into());
                    } else {
                        web_sys::console::log_1(&"No state available for rendering".into());
                    }
                    
                    // Request another frame
                    window.request_redraw();
                },
                WindowEvent::Resized(size) => {
                    if let Some(state) = &mut self.state {
                        state.resize(size);
                    }
                },
                WindowEvent::KeyboardInput { event, .. } => {
                    web_sys::console::log_1(&format!("Keyboard Event: {:?}", event).into());
                },
                _ => {
                    web_sys::console::log_1(&format!("Unrecognized Event: {:?}", event).into());
                }
            }
            return;
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            // For native, handle events normally
            let state = match &mut self.state {
                Some(state) => state,
                None => return,
            };
            
            if id != state.window().id() {
                return;
            }
            
            match event {
                WindowEvent::CloseRequested => {
                    println!("The close button was pressed; stopping");
                    event_loop.exit();
                },
                WindowEvent::RedrawRequested => {
                    match state.render() {
                        Ok(_) => {},
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.window().inner_size()),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => log::error!("render error: {e:?}"),
                    }
                    
                    // Emits a new redraw request
                    state.window().request_redraw();
                },
                WindowEvent::Resized(physical_size) => {
                    state.resize(physical_size);
                },
                WindowEvent::KeyboardInput { event, .. } => {
                    println!("Keyboard Event: {:?}", event);
                },
                _ => {
                    println!("Unrecognized Event: {:?}", event);
                }
            }
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
            web_sys::console::log_1(&"Starting web application".into());
        } else {
            env_logger::init();
            log::info!("Starting native application");
        }
    }

    let event_loop = EventLoop::new().unwrap();
    
    // When the current loop iteration finishes, immediately begin a new
    // iteration regardless of whether or not new events are available to
    // process. Preferred for applications that want to render as fast as
    // possible, like games.
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}