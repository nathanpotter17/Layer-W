# LAYER-W - Near-Native Web Execution Layer for Games & Applications

### Layer-W Overview & Purpose

- Overview: A tightly managed WebAssembly 3D engine that maximizes performance through aggressive memory reuse and graphics-oriented memory layout. The reserved memory model allows regioning to be flexible & granular - balancing rendering performance with necessary data persistence.

- Purpose: Pathfinding for eventual performant, platform agnostic application solution using WebGPU & Rust + WASM / WASI.

### Layer-W Stack

- rust, wasm_bindgen, cargo, bash, winit, wgpu, web_sys, js_sys, sdl2, Gamepad API, etc.

### Layer-W Goals

- Robust and Tuneable memory system, Mesh LOD / streaming based content system, integrated networking, multiplatform rendering, engine for A/AA Quality @ ~60FPS

### Layer-W Limitations

- Total Memory Limit is 4GB. Drawing system is limited to either OpenGL or WebGPU, the latter of which is still considered experimental. Fixed resolution TBD, but likely 1080 x 720. Max texture size is 1k, 2k maps used only on critical assets.

### Build System

- Cargo, Bash, wasm-bindgen, wasm-pack, wasmtime, wasm32-unknown-unknown & / or wasm32-wasip1/p2, x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu.

### Host Runtimes

- Windows, Linux, Browser, Wasmtime, WAMR https://github.com/bytecodealliance/wasm-micro-runtime/tree/main
- More Native platforms may be supported pending WASI improvements (TBD)

## Key Insights

The key is treating as much of the 4GB as possible as a high-speed circular buffer optimized for GPU consumption, not traditional memory.

- Think in GPU Cache Lines: Align necessary data to GPU-preferred boundaries
- Overwrite, Don't Allocate: Your ventilated approach is perfect
- Predict and Prefetch: Use velocity-based asset prediction
- Compress Aggressively: 50-70% reduction is achievable
- Cull Early and Often: GPU-based culling is fastest
- Use Web Workers: Parallel culling and decompression
- Adapt Quality Dynamically: Maintain 60fps above all
- Profile Everything: Measure, don't guess

## Memory Architecture

### WALLOC Module

- Always allocates full 4GB WASM memory space (large pre-allocation manager)
- No practical limits on desktop/web (mobile TBD)
- Memory regions tied directly to their respective functionality.
- Region Manager: Ventilated pool segmentation
- Overwrite Scheduler: Frame-based memory recycling
- Protection System: Core system memory guards

### WALLOC Technical Specifications

#### Memory Layout

- Total WASM memory: 4GB (walloc secured)
- Ventilated regions: ~3.8GB
- Protected system: ~200MB
- Page size: 64KiB
- Overwrite granularity: Configurable

#### Graphics Targets

- Resolution: 1920x1080
- Target FPS: 60
- Quality: AA-tier visuals
- Platform: Web + Native

#### Core Innovation

The ventilated memory model eliminates traditional allocation overhead by treating memory as a render-state machine rather than a general-purpose heap. Each frame "ventilates" the previous frame's allocations, creating a zero-overhead streaming system perfectly suited for real-time 3D graphics. The engine also ships with first class networking support, allowing assets to be streamed into memory efficiently via web workers. Side car patterns are also leveraged for easier integrative support.

### Memory Tiers

Graphics Pipeline Memory Layout:

- Mesh Pool: Dynamic LOD meshes (continuously recycled)
- Texture Pool: Streaming texture data (overwritten by distance)
- Command Pool: GPU command buffers (per-frame recreation)
- Scratch Pool: Temporary calculations (no persistence)
- System Pool: Core engine state (protected region)

## Graphics Pipeline

### Rendering System

- API: WebGPU via wgpu (web + native)
- Window: winit with raw_window_handle
- Surface: Platform-appropriate (HWND/NSWindow/X11/Canvas)

## Windowing & Input System

- Winit
  - Event Loop: Basic Input handling & frame timing
    - Window Management: Resize, fullscreen, focus
  - Surface Creation: Platform abstraction layer
- Input Handling (Side Car / Event Forwarding): SDL2 for native, Gamepad API for web.

## Rendering Systems

- wgpu forward rendered
- simple wgsl vertex and frag shaders to start

## Build System

- custom bash

### Completion Progress

- Completed
  - windowing, walloc, basic input/event loop
- Needs
  - walloc regioning, priority, alignment, cache system. Web Worker swarm, 4GB+ memory limit investigation.
  - Scene Management, UI, Networking, Audio, Physics, Animation, Asset Pipeline

### Memory-Coupled Features

- LOD meshes stored in ventilated pools
- Distance-based memory overwriting
- Frame-synchronized buffer recycling
- Direct GPU memory mapping - compute proper work blocks, consider compute shaders as an extension of WASMs processing ability.
- Aggressive region recycling
- Near-to-far overwrite pattern

### Threading/Concurrency

- Web Workers: Parallel processing
- Task System: Job scheduling
- Async Loading: Non-blocking I/O

## Asset Pipeline

- Model Loader: Mesh data streaming
- Texture Manager: Mipmap generation & compression
- Shader Compiler: WGSL/SPIR-V processing
- Asset Cache: Memory-mapped resource store
