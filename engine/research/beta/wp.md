# LAYER-W - Near-Native Web Execution Layer for Games & Applications

## Layer-W Overview

Overview: A tightly managed cross-platform application engine that maximizes performance through aggressive memory reuse and graphics-oriented memory layout. The reserved memory model balances rendering performance with necessary data persistence. Has the flexibility to render all content in a static SPA.

Purpose: Pathfinding for eventual performant, platform agnostic application solution using WebGPU & Rust + WASM / WASI.

Ethos: Layer-W has three core tenants; More with Less, Isogaba Maware, and Five S.

## Current Problem

Developers still expend massive resources creating separate codebases for each platform while platform fees drain publisher revenues, web-based games remain severely limited in fidelity forcing developers to choose between reach and quality, multi-gigabyte mandatory updates frustrate players while platform approval processes delay critical fixes, cross-platform systems remain fragmented despite player demand, and live-service games struggle with mounting server costs as complexity grows.

## Layer-W Thesis

As the world continues to become increasingly connected, the need for robust & secure applications grows rapidly. Layer-W is meant to meet this demand by simplifying the process of deploying to many platforms by leveraging Web Assembly and Rust to route application resource demands in a lightweight fashion.

Layer-W's hybrid native-WebAssembly architecture eliminates these pain points by enabling a single development pipeline that delivers native performance through WebGPU while maintaining web-level distribution, allowing developers to bypass costly gatekeepers, stream only necessary content, maintain version consistency across all devices, and create truly platform-agnostic experiences that can scale efficiently while preserving creative and economic freedoms.

## Layer-W Features

- Robust and Tuneable Memory System, Streaming based content system, Integrated networking, Multiplatform rendering, Application Engine for AA/AAA Quality Graphics @ ~60FPS.

## Layer-W Limitations

- Total Memory Limit is 4GB. Fixed resolution 1080 x 720. Max texture size is 2k, high quality maps should only be used only on critical assets.

## Host Runtimes

- Windows, Linux, Browser, Wasmtime, [WAMR](https://github.com/bytecodealliance/wasm-micro-runtime/tree/main)
- More Native platforms & functionality may be supported pending WASI improvements (TBD)

## Feature Parity

- Shared Business Logic: All core algorithms and business logic are in platform-agnostic Rust code.
- Platform Adapters: Adapter layers via cfg provide the same capabilities through different implementations.
- Feature Detection: Graceful degradation based on available features.

## Core Innovation

The ventilated memory model eliminates traditional allocation overhead by treating memory as a render-state machine rather than a general-purpose heap. Moreover, Memory regions tied directly to their respective functionality. Each frame "ventilates" the previous frame's allocations, creating a near zero-overhead streaming system perfectly suited for modern live service games with high quality graphics or enriched applications running on embedded devices, or in isolated contexts. The engine also ships with first class networking support, allowing assets to be streamed into memory efficiently. Side car patterns are also leveraged for easier integrative support where applicable.

## Rethinking Distribution

In order to serve the much larger crowd of tech users that exist in 2025; Layer-W puts distribution first - Apps don't load everything upfront; they stream components on-demand. Core engine remains lightweight (<5MB for initial boot) Assets, levels, and gameplay modules load progressively based on need. Think of it like a modern SPA but for apps, you don't load the entire app at once.Games become collections of small, cached components rather than monolithic binaries. Components can be reused across games (shared physics engines, rendering systems). Regional caching means components download once and work everywhere. Components can be updated independently without redownloading everything.

Modding support is available by default, although the ability to have components streamed into games is completely up to the developer to make those capability based decisions for their users.

## Execution benefits

Layer-W intelligently decides what runs locally vs remotely; Critical path (movement, aiming) runs locally. Heavy computation (AI, physics) streams from server. Physics can be split - core in native, secondary effects in WASM. Rendering pipeline is web enabled, and can be split between local and remote, as well as dynamically adjusted based on network conditions. Game logic can run in WASM (secure, portable, supports HMR). Layer-W gives the best of both worlds without compromise.

## Networking System

As the primary networking library, Layer-W uses reqwest, a WASM friendly batteries-included HTTP client, with extra help from wasm_bindgen_futures, to enable asset streaming and AOT scene loading from HTTP endpoints in the browser & from within Rust.

Because the browser effectively has a compiler that we can make use of, extra WASM code can be networked in, and compiled either AOT or on-device, without the need to hit any endpoints apart from the one delivering & potentially compiling the code. This could allow for many different HMR or other network oriented configurations for runtime code delivery. It also lends itself to the distribution first ethos of Layer-W. Despite the regions in Walloc's tiered allocator supporting growing to the max allocation size minus the persistent scene data, the main limitation here is still the download size.

## Graphics Pipeline

### Graphics Targets

- Resolution: 1080x720 (Support for 1920x1080 TBD)
- Target FPS: 60+
- Quality: AA/AAA-tier
- Platform: Web + Native

### Rendering System

- API: WebGPU via wgpu (web + native)
- Window: winit with raw_window_handle
- Surface: Platform-appropriate (HWND/NSWindow/X11/Canvas)

### Windowing & Input System

- Winit
  - Event Loop: Basic Input handling & frame timing
    - Window Management: Resize, fullscreen, focus
  - Surface Creation: Platform abstraction layer
- Input Handling (Side Car / Event Forwarding): SDL2 for native, Gamepad API for web.

## Up Next

- To Research: Walloc Cache System, Web Worker swarm - 4GB+ memory limit investigation.
- Next Up: Scene Management, UI, Audio, Physics, Animation.

## Advanced Features

### Memory-Coupled Features

- LOD meshes stored in ventilated pools
- Distance-based memory overwriting
- Frame-synchronized buffer recycling
- Direct GPU memory mapping - compute proper work blocks, consider compute shaders as an extension of WASMs processing ability.
- Aggressive region recycling
- Near-to-far overwrite pattern
