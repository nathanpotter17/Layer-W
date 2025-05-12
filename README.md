# LAYER-W - Near-Native Web Execution Layer for Games & Applications

Purpose: Pathfinding for eventual performant, platform agnostic application solution using WebGPU & Rust + WASM / WASI.

- Layer-W Stack
  - rust, wasm_bindgen, cargo, bash, web_sys, js_sys, wasm32-unknown-unknown, wasm32-wasip1/p2, x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu.
- Layer-W Goals
  - Robust and Tuneable memory system, Mesh LOD / streaming based content system, integrated networking, multiplatform rendering, engine for A/AA Quality @ ~60FPS
- Layer-W Limitations
  - Total Memory Limit is 4GB. Drawing system is limited to either OpenGL or WebGPU, the latter of which is still considered experimental. Fixed resolution TBD, but likely 1080 x 720. Max texture size is 1k, 2k maps used only on critical assets.
- Build System
  - Cargo, Bash, wasm-bindgen, wasm-pack, wasmtime, some native toolchains may be supported pending WASI GFX (TBD)
- Host Runtimes
  - Browser, Wasmtime, WAMR https://github.com/bytecodealliance/wasm-micro-runtime/tree/main, Native EXEs (Platforms TBD)

## Revision History

- Alpha
  - 4/22/25 - Alpha Proposal [v0.0.1a](/engine/research/pre-lim/layerwV0.0.1a.md)
  - 4/24/25 - Alpha Proposal Two [v0.0.2](/engine/research/pre-lim/layerwV0.0.2.md)
  - 4/26/25 - Alpha Proposal Three [v0.0.3](/engine/research/pre-lim/layerwV0.0.3.md)
- Beta
  - 5/5/25 - Beta Proposal [v0.1.0](/engine/research/beta/layerwV0.1.0.md)
    - Wevent, Winput Submodules Established (Alpha)
  - 5/8/25 - Beta Proposal Two [v0.1.1](/engine/research/beta/layerwV0.1.1.md)
    - Walloc Submodule Established (Alpha)
- Release
  - ?/?/?? - `TBD`

## Resources & Extras

- [The Complete WebAssembly Story - wasmCloud, Extism, Spin, and etc. Demos](https://www.youtube.com/watch?v=Wxw-YAGYHDc)
- [The Practical WASM Memory Guide](https://radu-matei.com/blog/practical-guide-to-wasm-memory/)
- [WASIX](https://wasix.org/) - The Superset of WASI, meant to be a community standard, long term stabilization and support of the existing WASI ABI plus additional non-invasive syscall extensions. Rust Toolchain, Wasmer Runtime.
- [Rust WebGPU GUI](https://github.com/zupzup/rust-wgpu-gui-example/tree/main)
- [wasmCloud - Build, manage, and scale Wasm apps across any cloud, K8s, or edge](https://wasmcloud.com/)
- [Rust Wasm Book](https://rustwasm.github.io/docs/book/why-rust-and-webassembly.html)
- [WASI & Component Model](https://www.youtube.com/watch?v=mkkYNw8gTQg)
- [Extism - The cross-language framework for building with WebAssembly using any language as a plugin](https://extism.org/)
  - [WASM, Extism in practice using Python (25:00)](https://www.youtube.com/watch?v=Wxw-YAGYHDc)
- [Emscripten & Wasm Zero to Hero](https://github.com/nathanpotter17/emcc-wasm)
- [PolyglotGPU - WebGPU in Three Languages](https://github.com/nathanpotter17/polyglot-gpu)
- [WASM I/O Youtube](https://www.youtube.com/@wasmio)
- [WASMs Future, Moving Away from Containers](https://www.youtube.com/watch?v=ZrLL6jrSVtk&t=480s)
- [Bytecode Alliance Projects](https://github.com/bytecodealliance)
- [State of WASM 2025, WASM Component Model](https://www.youtube.com/watch?v=KK0FKiQ7nis)
- [WasmPay as a reference architecture (platform harness, sidecar pattern) (11:00)](https://www.youtube.com/watch?v=FM2B8kYEasw)
- [WASM Components in Rust](https://component-model.bytecodealliance.org/language-support/rust.html)
- [Building components in Rust](https://component-model.bytecodealliance.org/language-support/rust.html)
- [Bytecode Alliance - Bytecode Alliance Mission](https://www.youtube.com/watch?v=ZrLL6jrSVtk)
- [Bytecode Alliance Active Projects](https://github.com/bytecodealliance/governance/tree/main/projects)
- [Bytecode Alliance Zulip Archive](https://github.com/bytecodealliance/zulip-archive)
- [WASI / WASM powered dev environments](https://www.youtube.com/watch?v=4bbU1gA2aSks)
- [HTTP / Async + Await in WASM: wstd crates.io](https://crates.io/crates/wstd)
- [Wasm Runtimes: Boxer](https://www.youtube.com/watch?v=rHOwhkHv21U)
- [WGPU](https://crates.io/crates/wgpu)
- [Wasm Component Model](https://component-model.bytecodealliance.org/)
- [WASI GFX](https://www.youtube.com/watch?v=HBJ1-S65bbM)
- [WebGPU Specification](https://www.w3.org/TR/webgpu/)
- [Bevy Engine (Hybrid Approach Example)](https://bevyengine.org/)
- [WASM-OPT -O3 if necessary - full paths only](https://rustwasm.github.io/book/reference/code-size.html)
- [K23 - microkernel w/ components, wasmtime runtime OS](https://www.youtube.com/watch?v=LraPUAV-fOo)

## Pre-Alpha - Previous WASM + Graphics Experiments

- [Intro to WASI](https://github.com/nathanpotter17/emcc-wasm/blob/main/src/wasi/README.md)
- [Rust Wasm ecosystem for simple WASI P1/P2 CLI apps](https://github.com/nathanpotter17/emcc-wasm/tree/main/src/wasi/rust/wasm-cla)
- [WIT, wasm shared memory, js module decl. from c++ via emscripten, image generation via typed array calls to/from c++ and canvas, emcc mastery and emcc vfs usage](https://github.com/nathanpotter17/emcc-wasm/tree/main/src/library)
- [Rust + wasm-bindgen + web_sys, js_sys for many simple demos](https://github.com/nathanpotter17/emcc-wasm/tree/main/src/wasi/rust/wasm-sys-bindgen)
- [Rust + wasm-bindgen + event handled input for a simple canvas game using web as host event loop](https://github.com/nathanpotter17/emcc-wasm/tree/main/src/wasi/rust/wasm-input)
- [Winit 0.29 + wgpu for drawing to canvas & natively](https://github.com/nathanpotter17/emcc-wasm/tree/main/src/wasi/rust/wasm-wgpu)
- [WebGPU in TS, glfw + vulkan on windows (C++), WebGPU Dawn (C++)](https://github.com/nathanpotter17/polyglot-gpu)
