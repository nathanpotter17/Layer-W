# LAYER-W - Near-Native Web Execution Layer for Games & Applications

### Overview

- A tightly managed cross-platform application engine that maximizes performance through aggressive memory reuse and graphics-oriented memory layout.
  The reserved memory model balances rendering performance with necessary data persistence. Has the flexibility to render all content in a static SPA.

### Purpose

- Pathfinding for performant, platform agnostic application solution using WebGPU & Rust + WASM / WASI.

### Layer-W Features

- Robust and Tuneable Memory System, Streaming based content system, Integrated networking, Multiplatform rendering, Application Engine for AA/AAA Quality @ ~60FPS

### Layer-W Limitations

- Total Memory Limit is 4GB. Fixed resolution 1080 x 720. Max texture size is 2k, high quality maps should only be used only on critical assets.

### Host Runtimes

- Windows, Linux, Browser, Wasmtime, [WAMR](https://github.com/bytecodealliance/wasm-micro-runtime/tree/main)
- More Native platforms & functionality may be supported pending WASI improvements (TBD)

### Feature Parity

- Shared Business Logic: All core algorithms and business logic are in platform-agnostic Rust code.
- Platform Adapters: Adapter layers via cfg provide the same capabilities through different implementations.
- Feature Detection: Graceful degradation based on available features.

### Core Innovation

The ventilated memory model eliminates traditional allocation overhead by treating memory as a render-state machine rather than a general-purpose heap. Moreover, Memory regions tied directly to their respective functionality. Each frame "ventilates" the previous frame's allocations, creating a near zero-overhead streaming system perfectly suited for modern live service games with high quality graphics or enriched applications running on embedded devices or in isolated contexts. The engine also ships with first class networking support, allowing assets to be streamed into memory efficiently via web workers. Side car patterns are also leveraged for easier integrative support.

### Memory Architecture

[Layer W Memory Module - Walloc](../../walloc/walloc.md)

### Networking System

Because the browser effectively has a compiler that we can make use of, extra wasm code can be networked in, and compiled on the client device, without the need to hit any endpoints apart from the one delivering the code. This could allow for many different HMR or other network oriented configurations for code compilation & delivery. It also lends itself to the distribution first ethos of Layer-W - This kind of availability would allow for networking in all kinds of functionality to Layer-W. The main limitation here is the download size.

### Graphics Pipeline

#### Graphics Targets

- Resolution: 1080x720 (Support for 1920x1080 TBD)
- Target FPS: 60+
- Quality: AA/AAA-tier
- Platform: Web + Native

#### Rendering System

- API: WebGPU via wgpu (web + native)
- Window: winit with raw_window_handle
- Surface: Platform-appropriate (HWND/NSWindow/X11/Canvas)

### Windowing & Input System

- Winit
  - Event Loop: Basic Input handling & frame timing
    - Window Management: Resize, fullscreen, focus
  - Surface Creation: Platform abstraction layer
- Input Handling (Side Car / Event Forwarding): SDL2 for native, Gamepad API for web.

![img](./img/LayerW.png)

### Up Next

- To Research: Walloc Cache System, Web Worker swarm - 4GB+ memory limit investigation.
- Next Up: Scene Management, UI, Asset Pipeline, Audio, Physics, Animation.

### Advanced Features

#### Memory-Coupled Features

- LOD meshes stored in ventilated pools
- Distance-based memory overwriting
- Frame-synchronized buffer recycling
- Direct GPU memory mapping - compute proper work blocks, consider compute shaders as an extension of WASMs processing ability.
- Aggressive region recycling
- Near-to-far overwrite pattern

#### Asset Pipeline

- Model Loader: Mesh data streaming
- Texture Manager: Mipmap generation & compression
- Shader Compiler: WGSL/SPIR-V processing
- Asset Cache: Memory-mapped resource store

#### Threading/Concurrency

- Web Workers: Parallel processing
- Task System: Job scheduling
- Async Loading: Non-blocking I/O
