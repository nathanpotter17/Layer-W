use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use winit::{
    application::ApplicationHandler,
    event::{WindowEvent, KeyEvent, MouseButton as WinitMouseButton},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
    keyboard::{PhysicalKey, KeyCode as WinitKeyCode},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

const DIMX: u32 = 1080;
const DIMY: u32 = 720;

// === EVENT SYSTEM ===

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    Tick,
    Quit,
    Input(InputEvent),
    Custom(Arc<str>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    KeyDown { key: KeyCode },
    KeyUp { key: KeyCode },
    MouseDown { button: MouseButton, x: f32, y: f32 },
    MouseUp { button: MouseButton, x: f32, y: f32 },
    MouseMove { x: f32, y: f32 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyCode {
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Digit0, Digit1, Digit2, Digit3, Digit4,
    Digit5, Digit6, Digit7, Digit8, Digit9,
    Space, Enter, Escape, Tab, Backspace,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Shift, Control, Alt,
    Unknown(Arc<str>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub timestamp: u64,
    pub data: Option<Arc<EventData>>,
}

#[derive(Debug, Clone)]
pub enum EventData {
    None,
    Integer(i64),
    Float(f64),
    Text(Arc<str>),
}

// Cross-platform timer
pub struct Timer {
    #[cfg(not(target_arch = "wasm32"))]
    start: Instant,
    #[cfg(target_arch = "wasm32")]
    start_time_ms: f64,
}

impl Timer {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                start: Instant::now(),
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            Self {
                start_time_ms: Self::now_ms(),
            }
        }
    }

    pub fn elapsed(&self) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start.elapsed()
        }

        #[cfg(target_arch = "wasm32")]
        {
            let now_ms = Self::now_ms();
            let elapsed_ms = now_ms - self.start_time_ms;
            Duration::from_secs_f64(elapsed_ms / 1000.0)
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed().as_millis() as u64
    }

    pub fn reset(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start = Instant::now();
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.start_time_ms = Self::now_ms();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn now_ms() -> f64 {
        use js_sys::Date;
        Date::now()
    }
}

// Event system
pub struct WEvent {
    event_queue: VecDeque<Arc<Event>>,
    timer: Timer,
    last_tick_time: u64,
    tick_interval_ms: u64,
}

impl WEvent {
    pub fn new() -> Self {
        Self {
            event_queue: VecDeque::new(),
            timer: Timer::new(),
            last_tick_time: 0,
            tick_interval_ms: 16, // ~60 FPS by default
        }
    }

    pub fn with_tick_rate(fps: u32) -> Self {
        let tick_interval_ms = if fps > 0 { 1000_u64 / fps as u64 } else { 16 };
        Self {
            event_queue: VecDeque::new(),
            timer: Timer::new(),
            last_tick_time: 0,
            tick_interval_ms,
        }
    }

    pub fn push_event(&mut self, event_type: EventType, data: Option<Arc<EventData>>) {
        let event = Arc::new(Event {
            event_type,
            timestamp: self.timer.elapsed_ms(),
            data,
        });
        self.event_queue.push_back(event);
    }

    pub fn poll_event(&mut self) -> Option<Arc<Event>> {
        self.event_queue.pop_front()
    }

    pub fn update(&mut self) {
        let current_time = self.timer.elapsed_ms();
        
        if current_time - self.last_tick_time >= self.tick_interval_ms {
            self.last_tick_time = current_time;
            self.push_event(EventType::Tick, Some(Arc::new(EventData::Integer(current_time as i64))));
        }
    }

    pub fn clear_events(&mut self) {
        self.event_queue.clear();
    }

    pub fn has_events(&self) -> bool {
        !self.event_queue.is_empty()
    }

    pub fn event_count(&self) -> usize {
        self.event_queue.len()
    }

    pub fn set_tick_interval(&mut self, interval_ms: u64) {
        self.tick_interval_ms = interval_ms;
    }

    pub fn timer(&self) -> &Timer {
        &self.timer
    }

    pub fn timer_mut(&mut self) -> &mut Timer {
        &mut self.timer
    }
}

// === INPUT SYSTEM ===

pub struct InputHandler {
    mouse_position: (f32, f32),
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            mouse_position: (0.0, 0.0),
        }
    }

    pub fn handle_winit_event(&mut self, event: &WindowEvent, wevent: &mut WEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_event(event, wevent);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_button(*state, *button, wevent);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = (position.x as f32, position.y as f32);
                wevent.push_event(
                    EventType::Input(InputEvent::MouseMove { 
                        x: self.mouse_position.0, 
                        y: self.mouse_position.1 
                    }), 
                    None
                );
            }
            WindowEvent::CloseRequested => {
                wevent.push_event(EventType::Quit, None);
            }
            _ => {}
        }
    }

    fn handle_keyboard_event(&mut self, event: &KeyEvent, wevent: &mut WEvent) {
        let key_code = self.convert_winit_keycode(&event.physical_key);
        
        let input_event = match event.state {
            winit::event::ElementState::Pressed => {
                InputEvent::KeyDown { key: key_code }
            }
            winit::event::ElementState::Released => {
                InputEvent::KeyUp { key: key_code }
            }
        };
        
        wevent.push_event(EventType::Input(input_event), None);
    }

    fn handle_mouse_button(&mut self, state: winit::event::ElementState, button: WinitMouseButton, wevent: &mut WEvent) {
        let mouse_button = self.convert_winit_mouse_button(button);
        let (x, y) = self.mouse_position;

        let input_event = match state {
            winit::event::ElementState::Pressed => {
                InputEvent::MouseDown { button: mouse_button, x, y }
            }
            winit::event::ElementState::Released => {
                InputEvent::MouseUp { button: mouse_button, x, y }
            }
        };
        
        wevent.push_event(EventType::Input(input_event), None);
    }

    fn convert_winit_keycode(&self, physical_key: &PhysicalKey) -> KeyCode {
        match physical_key {
            PhysicalKey::Code(code) => match code {
                WinitKeyCode::KeyA => KeyCode::A,
                WinitKeyCode::KeyB => KeyCode::B,
                WinitKeyCode::KeyC => KeyCode::C,
                WinitKeyCode::KeyD => KeyCode::D,
                WinitKeyCode::KeyE => KeyCode::E,
                WinitKeyCode::KeyF => KeyCode::F,
                WinitKeyCode::KeyG => KeyCode::G,
                WinitKeyCode::KeyH => KeyCode::H,
                WinitKeyCode::KeyI => KeyCode::I,
                WinitKeyCode::KeyJ => KeyCode::J,
                WinitKeyCode::KeyK => KeyCode::K,
                WinitKeyCode::KeyL => KeyCode::L,
                WinitKeyCode::KeyM => KeyCode::M,
                WinitKeyCode::KeyN => KeyCode::N,
                WinitKeyCode::KeyO => KeyCode::O,
                WinitKeyCode::KeyP => KeyCode::P,
                WinitKeyCode::KeyQ => KeyCode::Q,
                WinitKeyCode::KeyR => KeyCode::R,
                WinitKeyCode::KeyS => KeyCode::S,
                WinitKeyCode::KeyT => KeyCode::T,
                WinitKeyCode::KeyU => KeyCode::U,
                WinitKeyCode::KeyV => KeyCode::V,
                WinitKeyCode::KeyW => KeyCode::W,
                WinitKeyCode::KeyX => KeyCode::X,
                WinitKeyCode::KeyY => KeyCode::Y,
                WinitKeyCode::KeyZ => KeyCode::Z,
                WinitKeyCode::Digit0 => KeyCode::Digit0,
                WinitKeyCode::Digit1 => KeyCode::Digit1,
                WinitKeyCode::Digit2 => KeyCode::Digit2,
                WinitKeyCode::Digit3 => KeyCode::Digit3,
                WinitKeyCode::Digit4 => KeyCode::Digit4,
                WinitKeyCode::Digit5 => KeyCode::Digit5,
                WinitKeyCode::Digit6 => KeyCode::Digit6,
                WinitKeyCode::Digit7 => KeyCode::Digit7,
                WinitKeyCode::Digit8 => KeyCode::Digit8,
                WinitKeyCode::Digit9 => KeyCode::Digit9,
                WinitKeyCode::Space => KeyCode::Space,
                WinitKeyCode::Enter => KeyCode::Enter,
                WinitKeyCode::Escape => KeyCode::Escape,
                WinitKeyCode::Tab => KeyCode::Tab,
                WinitKeyCode::Backspace => KeyCode::Backspace,
                WinitKeyCode::ArrowUp => KeyCode::ArrowUp,
                WinitKeyCode::ArrowDown => KeyCode::ArrowDown,
                WinitKeyCode::ArrowLeft => KeyCode::ArrowLeft,
                WinitKeyCode::ArrowRight => KeyCode::ArrowRight,
                WinitKeyCode::ShiftLeft | WinitKeyCode::ShiftRight => KeyCode::Shift,
                WinitKeyCode::ControlLeft | WinitKeyCode::ControlRight => KeyCode::Control,
                WinitKeyCode::AltLeft | WinitKeyCode::AltRight => KeyCode::Alt,
                _ => KeyCode::Unknown(Arc::from(format!("{:?}", code))),
            },
            _ => KeyCode::Unknown(Arc::from("Unknown")),
        }
    }

    fn convert_winit_mouse_button(&self, button: WinitMouseButton) -> MouseButton {
        match button {
            WinitMouseButton::Left => MouseButton::Left,
            WinitMouseButton::Right => MouseButton::Right,
            WinitMouseButton::Middle => MouseButton::Middle,
            WinitMouseButton::Back => MouseButton::Other(3),
            WinitMouseButton::Forward => MouseButton::Other(4),
            WinitMouseButton::Other(id) => MouseButton::Other(id),
        }
    }
}

// === WINDOWING SYSTEM ===

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> State {
        // Configure instance based on platform
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let size = winit::dpi::PhysicalSize::new(DIMX, DIMY);
                let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                    backends: wgpu::Backends::BROWSER_WEBGPU,
                    ..Default::default()
                });
                let limits = wgpu::Limits::downlevel_webgl2_defaults();
            } else {
                let size = window.inner_size();
                let instance = wgpu::Instance::default();
                let limits = wgpu::Limits::default();
            }
        }

        let surface = instance.create_surface(window.clone()).expect("Failed to create surface");

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

        let adapter_info = adapter.get_info();

        #[cfg(not(target_arch = "wasm32"))]
        println!("Selected GPU: {} ({:?})", adapter_info.name, adapter_info.device_type);

        #[cfg(target_arch = "wasm32")]
        {
            let js_code = r#"
                if (navigator.gpu) {
                    console.log("WebGPU is supported");
                    console.log("Hardware concurrency: " + navigator.hardwareConcurrency);
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
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
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
        
        let state = State::new(self.window.clone()).await;
        
        web_sys::console::log_1(&"State initialized, updating App...".into());
        
        unsafe {
            let app = &mut *self.app_ptr;
            app.state = Some(state);
            app.state_initializing = false;
            
            web_sys::console::log_1(&"App state updated!".into());
        }
    }
}

// === APPLICATION ===

#[derive(Default)]
struct App {
    state: Option<State>,
    window: Option<Arc<Window>>,
    wevent: Option<WEvent>,
    input_handler: Option<InputHandler>,
    #[cfg(target_arch = "wasm32")]
    state_initializing: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Multiplatform Window")
                        .with_inner_size(winit::dpi::PhysicalSize::new(DIMX, DIMY))
                )
                .unwrap(),
        );

        window.set_min_inner_size(Some(winit::dpi::PhysicalSize::new(DIMX, DIMY)));
        window.set_max_inner_size(Some(winit::dpi::PhysicalSize::new(DIMX, DIMY)));
        window.set_resizable(false);
        
        // Initialize event system and input handler
        self.wevent = Some(WEvent::with_tick_rate(60));
        self.input_handler = Some(InputHandler::new());
        
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            web_sys::console::log_1(&"Setting up web canvas".into());

            let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(DIMX, DIMY));
            
            if let Some(canvas) = window.canvas() {
                let web_window = web_sys::window().unwrap();
                let document = web_window.document().unwrap();
                
                let container = document.get_element_by_id("app")
                    .unwrap_or_else(|| document.body().unwrap().into());
                
                canvas.set_width(DIMX.into());
                canvas.set_height(DIMY.into());
                
                let style = canvas.style();
                style.set_property("width", &format!("{}px", DIMX)).unwrap();
                style.set_property("height", &format!("{}px", DIMY)).unwrap();
                style.set_property("max-width", &format!("{}px", DIMX)).unwrap();
                style.set_property("max-height", &format!("{}px", DIMY)).unwrap();
                
                container.append_child(&web_sys::Element::from(canvas))
                    .expect("Couldn't append canvas to document");
                
                web_sys::console::log_1(&"Canvas attached to document".into());
            }
            
            self.window = Some(window.clone());
            self.state_initializing = true;
            
            let initializer = StateInitializer::new(window.clone(), self);
            wasm_bindgen_futures::spawn_local(initializer.initialize());
            
            window.request_redraw();
            return;
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let state = pollster::block_on(State::new(window.clone()));
            self.state = Some(state);
            self.window = Some(window.clone());
            window.request_redraw();
        }
    }

   fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // Handle input events through our input handler
        if let (Some(input_handler), Some(wevent)) = (&mut self.input_handler, &mut self.wevent) {
            input_handler.handle_winit_event(&event, wevent);
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            // WASM-specific handling with state_initializing check
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
                    // Update event system and collect events to process
                    let mut events_to_process = Vec::new();
                    if let Some(wevent) = &mut self.wevent {
                        wevent.update();
                        
                        // Collect all events first
                        while let Some(game_event) = wevent.poll_event() {
                            events_to_process.push(game_event);
                        }
                    }

                    // Process collected events
                    for game_event in events_to_process {
                        self.handle_game_event(&game_event);
                    }

                    // If state is initialized, render
                    if let Some(state) = &mut self.state {
                        match state.render() {
                            Ok(_) => {},
                            Err(wgpu::SurfaceError::Lost) => {
                                web_sys::console::warn_1(&"Surface lost, reconfiguring...".into());
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
                    
                    // Get a fresh borrow for request_redraw
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                },
                _ => {}
            }
            return;
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native platform handling (no state_initializing needed)
            let window = match &self.window {
                Some(window) => window,
                None => return,
            };
            
            if window.id() != id {
                return;
            }
            
            match event {
                WindowEvent::CloseRequested => {
                    println!("The close button was pressed; stopping");
                    event_loop.exit();
                },
                WindowEvent::RedrawRequested => {
                    // Update event system and collect events to process
                    let mut events_to_process = Vec::new();
                    if let Some(wevent) = &mut self.wevent {
                        wevent.update();
                        
                        // Collect all events first
                        while let Some(game_event) = wevent.poll_event() {
                            events_to_process.push(game_event);
                        }
                    }

                    // Process collected events
                    for game_event in events_to_process {
                        self.handle_game_event(&game_event);
                    }

                    let state = match &mut self.state {
                        Some(state) => state,
                        None => return,
                    };

                    match state.render() {
                        Ok(_) => {},
                        Err(wgpu::SurfaceError::Lost) => println!("Surface lost..."),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => log::error!("render error: {e:?}"),
                    }

                    // Get a fresh borrow for request_redraw
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                },
                _ => {}
            }
        }
    }
}

impl App {
    fn handle_game_event(&mut self, event: &Arc<Event>) {
        match &event.event_type {
            EventType::Tick => {
                // Handle game tick
            }
            EventType::Quit => {
                // Handle quit request
            }
            EventType::Input(input_event) => {
                match input_event {
                    InputEvent::KeyDown { key } => {
                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(&format!("Key pressed: {:?}", key).into());
                        #[cfg(not(target_arch = "wasm32"))]
                        println!("Key pressed: {:?}", key);
                    }
                    InputEvent::MouseDown { button, x, y } => {
                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(&format!("Mouse clicked: {:?} at ({}, {})", button, x, y).into());
                        #[cfg(not(target_arch = "wasm32"))]
                        println!("Mouse clicked: {:?} at ({}, {})", button, x, y);
                    }
                    _ => {}
                }
            }
            EventType::Custom(name) => {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("Custom event: {}", name).into());
                #[cfg(not(target_arch = "wasm32"))]
                println!("Custom event: {}", name);
            }
        }
    }

    // Helper methods for creating events with proper Arc usage
    pub fn push_custom_event(&mut self, name: impl Into<Arc<str>>) {
        if let Some(wevent) = &mut self.wevent {
            wevent.push_event(EventType::Custom(name.into()), None);
        }
    }

    pub fn push_custom_event_with_text(&mut self, name: impl Into<Arc<str>>, text: impl Into<Arc<str>>) {
        if let Some(wevent) = &mut self.wevent {
            wevent.push_event(
                EventType::Custom(name.into()),
                Some(Arc::new(EventData::Text(text.into()))),
            );
        }
    }

    pub fn push_custom_event_with_number(&mut self, name: impl Into<Arc<str>>, value: i64) {
        if let Some(wevent) = &mut self.wevent {
            wevent.push_event(
                EventType::Custom(name.into()),
                Some(Arc::new(EventData::Integer(value))),
            );
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
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}