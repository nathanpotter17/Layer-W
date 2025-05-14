# SUBMODULE_004: WWINDOW: A WebAssembly friendly window and rendering using Rust

To address different windowing, rendering, and surfacing approaches between web and native:

- Window Management: Winit creates and manages windows, handling platform-specific details for desktop and web environments
- Event Loop: Provides a unified event system for user input (keyboard, mouse, touch), window events (resize, focus, close), and system events
- Platform Abstraction: Seamlessly abstracts OS-specific windowing APIs behind a consistent interface

## Graphics API

- Minimal overhead - it's a thin wrapper around platform APIs
- Efficient event batching and coalescing
- No hidden memory costs or background threads

## Platform Implementations

Unlike many window management libraries, winit has first-class web support. It translates web events (mouse, keyboard, touch, gamepad) into a unified event system that works identically across all platforms. Winit implements the raw_window_handle::HasWindowHandle and raw_window_handle::HasDisplayHandle traits under the hood, which is perfect for our usecase.

- Windows: Provides HWND handle for Win32 windows
- macOS: Provides NSWindow handle for Cocoa windows
- Linux: Provides X11 Window ID or Wayland surface handle
- Web/WASM: Provides HTML canvas element handle

The graphics library (like WGPU) takes Winit's handle and creates the platform-appropriate surface (Direct3D/Vulkan on Windows, Metal on macOS, Vulkan on Linux, WebGPU on web).

The overhead is front-loaded during initialization, then it gets out of the way for actual rendering. Use #[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]

# WebGPU Browser Compatibility Approaches and Errors

This document contains various experimental approaches and debugging techniques used to resolve WebGPU compatibility issues in browsers.

## Errors

```
Uncaught (in promise) JsValue(OperationError: Failed to execute 'requestDevice' on 'GPUAdapter': The limit "maxInterStageShaderComponents" with a non-undefined value is not recognized.

Another issue - creating the rendering surface. may need to use winit for web_sys windowing support, implementing from scratch using rwh was not trivial.
```

## Different Limits Approaches Tried

We provide three different defaults.

- Limits::downlevel_defaults(). This is a set of limits that is guaranteed to work on almost all backends, including “downlevel” backends such as OpenGL and D3D11, other than WebGL. For most applications we recommend using these limits, assuming they are high enough for your application, and you do not intent to support WebGL.

- Limits::downlevel_webgl2_defaults() This is a set of limits that is lower even than the downlevel_defaults(), configured to be low enough to support running in the browser using WebGL2.

- Limits::default(). This is the set of limits that is guaranteed to work on all modern backends and is guaranteed to be supported by WebGPU. Applications needing more modern features can use this as a reasonable set of limits if they are targeting only desktop and modern mobile devices.

We recommend starting with the most restrictive limits you can and manually increasing the limits you need boosted. This will let you stay running on all hardware that supports the limits you need.

Limits “better” than the default must be supported by the adapter and requested when requesting a device. If limits “better” than the adapter supports are requested, requesting a device will panic. Once a device is requested, you may only use resources up to the limits requested even if the adapter supports “better” limits.

Requesting limits that are “better” than you need may cause performance to decrease because the implementation needs to support more than is needed. You should ideally only request exactly what you need.

### 1. Default Limits (Failed)

```rust
let limits = wgpu::Limits::default();
```

Error: `maxInterStageShaderComponents` limit not recognized by browsers.

### 2. Downlevel WebGL2 Defaults (Success)

```rust
let limits = wgpu::Limits::downlevel_webgl2_defaults();
```

This worked - provides minimal WebGL2-compatible limits.

### 3. Custom Conservative Limits

```rust
let limits = if cfg!(target_arch = "wasm32") {
    wgpu::Limits {
        max_texture_dimension_1d: 8192,
        max_texture_dimension_2d: 8192,
        max_texture_dimension_3d: 2048,
        max_texture_array_layers: 256,
        max_bind_groups: 4,
        // ... many more fields
        ..Default::default()
    }
} else {
    wgpu::Limits::default()
};
```

This approach was too complex and still had compatibility issues.

### 4. Using Adapter Limits

```rust
let limits = wgpu::Limits::downlevel_webgl2_defaults()
    .using_resolution(adapter.limits());
```

Attempted to use the intersection of default limits and adapter capabilities.

### 5. Empty Device Descriptor

```rust
let device_desc = if cfg!(target_arch = "wasm32") {
    wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(), // Let browser decide
    }
}
```

Still had issues with unrecognized limits.

## Backend Selection Experiments

### 1. All Backends (Original)

```rust
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::all(),
    ..Default::default()
});
```

### 2. WebGL Only for WASM (Success)

```rust
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::GL,
    ..Default::default()
});
```

This was crucial - forces WebGL backend which is more compatible.

### 3. WebGPU with WebGL Fallback

```rust
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: if cfg!(target_arch = "wasm32") {
        wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL
    } else {
        wgpu::Backends::all()
    },
    ..Default::default()
});
```

## Surface Creation Methods

### 1. Safe Surface Creation (Original)

```rust
let surface = instance.create_surface(window.clone()).unwrap();
```

### 2. Unsafe Surface Creation (Success)

```rust
let surface = unsafe {
    instance.create_surface_unsafe(
        wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap()
    )
}.expect("Failed to create surface");
```

The working example used this approach, which may handle edge cases better.

## Key Findings

1. **WebGL Backend is Essential**: For web targets, explicitly using `wgpu::Backends::GL` is crucial.
2. **WebGL Feature Flag**: The `webgl` feature must be enabled in Cargo.toml for wgpu.
3. **Downlevel Limits**: Use `wgpu::Limits::downlevel_webgl2_defaults()` for web compatibility.
4. **Surface Creation**: The unsafe method seems more reliable for cross-platform support.
5. **Version Compatibility**: Specific versions of wgpu/winit work better together.

## Browser Considerations

- Chrome/Edge: Generally good WebGPU support, but may need flags enabled
- Firefox: WebGPU support is experimental
- Safari: Limited WebGPU support, WebGL fallback important

## Debugging Tips

1. Always check browser console for detailed error messages
2. Use the debug HTML page to verify WebGPU availability
3. Test with different browsers and versions
4. Enable browser developer flags for WebGPU if needed
5. Consider WebGL fallback for broader compatibility

## Final Working Configuration

```toml
[dependencies]
wgpu = { version = "0.19.3", features = ["webgl"]}
```

```rust
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..Default::default()
        });
        let limits = wgpu::Limits::downlevel_webgl2_defaults();
    } else {
        let instance = wgpu::Instance::default();
        let limits = wgpu::Limits::default();
    }
}
```

This configuration ensures maximum compatibility across platforms while maintaining simplicity.
