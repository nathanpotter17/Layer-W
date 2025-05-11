# LAYER-W - Near Native Web Execution Layer for Games & Applications

Layer Stack: rust, wasm_bindgen, cargo, web_sys, js_sys, wasm32-unknown-unknown, wasm32-wasip1/p2, bash

Layer Goals: Robust and Tuneable memory + Mesh LOD / streaming system = A/AA Shading Quality @ ~60FPS

Layer Limits: Total Memory Limit is advertised as 4GB, however, available memory is only 2GB in Chrome, workers can sometimes get the full 4GB... Drawing system is limited to either OpenGL or WebGPU, the latter of which is still considered experimental. Fixed resolution TBD, but likely 1080 x 720. [WASM Memory Alloc Issue](https://users.rust-lang.org/t/chrome-wasm32-4gb-2gb-limits-workarounds/78161/5)

Optimize for WASM's 32-bit address space, and page style memory allocation.

Need to think about integrating Binaryen into the build system.

Host Runtimes: Browser, Wasmtime, WAMR https://github.com/bytecodealliance/wasm-micro-runtime/tree/main

- LayerW: Open World, UE-esque
  - Asset Pipeline
    - asset loading system is LOD / streaming based - apart from small static definitions
      - Memory and Game Data is represented as Float32Array or similar typed array whenever possible
  - Architecture & Build System
    - Build uses Cargo, Bash, wasm-bindgen, wasm-pack, wasmtime, some native toolchains may be supported pending WASI GFX (TBD).
    - Architecture is Component based, Streaming oriented, distribution first.
- Memory: Wasm shared memory, memory views, constraints, and rust data management + types

  - Pre-optimized memory pools established AOT. (Re-cycled continuously, Achieved 4GB max limit through gradual growth)

    - Circumvent memory contraints by utilizing `webworker, js_sys::ArrayBuffer, or js_sys::SharedArrayBuffer`.
    - Emscripten and its workaround for memory limits: [V8 Dev Blog](https://v8.dev/blog/4gb-wasm-memory)
    - [Chromium Single Tab Memory Limits](https://groups.google.com/a/chromium.org/g/chromium-dev/c/IKZvzuBP9QE/m/caF-Yge4AwAJ)
    - Consider using a "Ventilated" Allocator (Heap is overwritten continously)

  - Instancing, SIMD, and other memory improvements are _highly preferred_ in WASM.
    - Allocate dedicated memory regions for different Layer systems, Organize memory by object lifetime expectations and based on usage size - Small objects (≤64 bytes), medium objects (65-1024 bytes), large objects (>1KB)
      - De-fragmentation routines may be necessary for large games, especially for large single levels.
  - Memory chunks are 64KiB, the max size of a WASM Page.
  - For extra optimizations, BVH or similar algo can be used to render maximum LOD in a smaller area.
  - 2^16 pages is 2^16 \* 64KiB = ~ 4GiB (4194304 bytes), which is the maximum range that a Wasm module can address, as Wasm currently only allows 32-bit addressing (address space is 2^32).

    - Games with large levels stream in content from within its current memory region - think Minecraft, or tile-based games.

- Rendering

  - WebGPU or wgpu, canvas is rendering surface. OR, Ideally [WASI GFX](https://github.com/WebAssembly/wasi-gfx) or [Wander GPU Framework](https://github.com/renderlet/wander) which are highly experimental (April 2025) and use the WASI ABI convention, allowing LayerW to use SDL2 exclusively.

    - Games will rival visual quality comparable to mid-tier AA games from a few years ago, or stylized games with more modern rendering approaches.
    - Windowing, input & sound: SDL2 on native, Gamepad Web API
    - Rendering approach could boil down to this CPP alternative, used in a previous project. Web Sys or similar may have [GPU functions available](https://rustwasm.github.io/wasm-bindgen/api/web_sys/gpu_buffer_usage/index.html);

      - Reasoning (found this out organically from Emscripten usage) [Source](https://users.rust-lang.org/t/maximal-webgl2-ram-a-rust-wasm32-program-can-use-on-latest-chrome/72457/4)

        ```
          You can load an ArrayBuffer, and pass that to buffer_data_with_array_buffer_view, all without allocating any memory in wasm itself. The limits here are going to be identical to whatever plain JS and WebGL can do.

          Similarly, you can create texture objects from HtmlImageElements, within wasm code, without actually loading those images as bitmap data into wasm memory.

          Otoh if you want to load a huge Vec<u8> on the Rust side, you are limited by wasm memory constraints, whether you intend to upload that as a WebGL buffer or not.
        ```

      - ```c++
          // See https://emscripten.org/docs/api_reference/module.html#creating-the-module-object for interacting with the Module object, however exporting
          // this function adds overhead instead of just calling the function directly here using the EM_ASM macro. Also could lead to race conditions.
          EM_ASM({
              const canvas = Module.canvas;
              const ctx = canvas.getContext('2d', { willReadFrequently: true });

              // Directly access the memory buffer from JavaScript
              // https://emscripten.org/docs/porting/connecting_cpp_and_javascript/Interacting-with-code.html#access-memory-from-javascript

              const imgData = new ImageData(new Uint8ClampedArray(HEAPU8.subarray($0, $0 + $1)), $2, $3);
              ctx.putImageData(imgData, 0, 0);
              console.log("Frame rendered");
          }, rgbaFrame.data, FRAME_SIZE, FRAME_WIDTH, FRAME_HEIGHT);
        ```

Next Steps:

- Write an allocator for WASM memory in Rust &check;
- Make it a tiered allocator, for each of the memory regions needed for level functioning. Level logic, meshes, textures, etc.
  - While the rest of the core engine functions live in stack memory, our allocator always measures itself first, and offsets from there as the start so that memory regions can stay protected.
    - Later: Make the tiered allocator tied to the distance function on the player controller's camera during rendering to cull assets...
