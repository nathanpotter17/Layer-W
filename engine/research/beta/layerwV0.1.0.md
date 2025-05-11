# LAYER-W - Near Native Web Execution Layer for Games & Applications

## essentially just https://github.com/renderlet/wander

Implemented:

- SUBMODULE_001: LAYER-W/ENGINE/wevent.rs - Cross platform event and game loop system. Uses VecDeque for event system. Has a Timer as well, uses WASI P2 Monotonic Clock when targeting WASI.

- SUBMODULE_002: LAYER-W/ENGINE/winput.rs - Cross platform input handling system. Uses SDL2 Events (or Gamepad API events) mapped to my WEvent::CustomEvent. Detects connection, disconnect, and all mapped inputs.

Engine Stack: rust, wasm_bindgen, cargo, web_sys, js_sys, wasm32-unknown-unknown, wasm32-wasip1/p2, bash

- open world engine
  - asset loading system is streaming based - apart from small static definitions
    - Memory and Game Data is represented as Float32Array wherever possible
- Type System - Rust + Wasm Bindgen, WASM Components, WIT as IDL.
- Networking System - WASI-HTTP & WASI Preview 2 IPC
- Event System - wevent.rs (simple tick & event system) + event listeners / reactor pattern.
- memory - wasm shared memory buffer / rust data types + management
  - Pre-optimized memory pools established AOT. (Re-cycled continuously)
  - Instancing, SIMD, and other memory improvements are _highly preferred_ in WASM.
    - Allocate dedicated memory regions for different engine systems, Organize memory by object lifetime expectations and based on usage size - Small objects (≤64 bytes), medium objects (65-1024 bytes), large objects (>1KB)
      - De-fragmentation routines may be necessary for large games.
  - (chunks are 64KiB, the max size of a WASM Page.)
  - 2^16 pages is 2^16 \* 64KiB = 4GiB bytes, which is the maximum range that a Wasm module can address, as Wasm currently only allows 32-bit addressing.
    - Games with large levels stream in content from within its current memory region - think Minecraft, or tile-based games.
    - Games with visual quality comparable to mid-tier AA games from a few years ago, or stylized games with more modern rendering approaches.
- architecture - component based, streaming + distribution first.
- graphics - webGPU or wgpu, canvas is rendering surface. OR, Ideally https://github.com/WebAssembly/wasi-gfx / https://github.com/renderlet/wander
  - windowing, input & sound: SDL2 on native, Gamepad Web API
- Build system: Cargo, Bash, wasm-bindgen, wasm-pack, wasmtime.
