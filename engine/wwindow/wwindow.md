# SUBMODULE_004: WWINDOW: A WebAssembly friendly window and rendering using Rust

To address different windowing, rendering, and surfacing approaches between web and native:

- Window Management: Winit creates and manages windows, handling platform-specific details for desktop and web environments
- Event Loop: Provides a unified event system for user input (keyboard, mouse, touch), window events (resize, focus, close), and system events
- Thin Networking Layer: Wry allows an embeddable webview on native to effectively enable networking in a K.I.S.S. format, while fetch() is available for the browser version as a web standard via js_sys / web_sys.
- Platform Abstraction: Seamlessly abstracts OS-specific windowing APIs behind a consistent interface

## Platform Implementations

Unlike many window management libraries, winit has first-class web support through wry, which provides a native web view, and web_sys & js_sys for browser WASM contexts. It translates web events (mouse, keyboard, touch, gamepad) into a unified event system that works identically across all platforms. Winit implements the raw_window_handle::HasWindowHandle and raw_window_handle::HasDisplayHandle traits under the hood, which is perfect for our usecase.

- Wry: Accepts Winit's Raw Window Handle Implementations for Native Chromium WebView
  - Windows: Provides HWND handle for Win32 windows
  - macOS: Provides NSWindow handle for Cocoa windows
  - Linux: Provides X11 Window ID or Wayland surface handle
  - Web/WASM: Provides HTML canvas element handle
    - Integrated Networking via web_sys, js_sys fetch()

The graphics library (like WGPU) takes Winit's Raw Window Handle and creates the platform-appropriate surface (Direct3D/Vulkan on Windows, Metal on macOS, Vulkan on Linux, WebGPU on web).

The overhead is front-loaded during initialization, then it gets out of the way for actual rendering. Use #[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]

## Graphics API - WGPU

- Minimal overhead - it's a thin wrapper around platform APIs
- Efficient & Transparent
- No hidden memory costs or background threads

## Usage Notes

- WebGPU on Windows with Chrome requires you to set your preferred device to the high performance method in you GPU manager and in the windows graphics settings.
- You will also need to enable experimental webgpu flags in chrome.

## WGPU Implementation

This configuration ensures maximum compatibility across platforms while maintaining simplicity.

```toml
[dependencies]
winit = "0.30.8"
wgpu = { version = "25.0.0", features = ["webgpu"]}
wry = "0.51.2"
```

```rust
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
        let limits = wgpu::Limits::downlevel_webgl2_defaults();
    } else {
        let instance = wgpu::Instance::default();
        let limits = wgpu::Limits::default();
    }
}
```
