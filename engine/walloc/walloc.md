# SUBMODULE_001: WALLOC: A WebAssembly memory allocator using Rust

Walloc is a custom memory allocator implemented in Rust for WebAssembly applications, specifically optimized for 3D graphics rendering engines. It provides efficient memory management with direct control over the WASM linear memory space, enabling high-performance memory utilization within browser environments.

## Key Features

- Efficient Memory Utilization: Intelligently manages WebAssembly's linear memory, gradually growing as needed up to near the full 4GB address space.
- Direct Memory Access: Provides low-level memory manipulation with typed array views, supporting raw memory operations critical for graphics applications.
- Configurable Allocation Strategy: Uses a first-fit allocation strategy for speed with block splitting and coalescing to minimize fragmentation.
- Memory Lifecycle Management: Supports allocation, deallocation, reallocation, and complete memory reset functionality.
- Browser Compatibility: Designed to work around browser-specific memory limitations while maximizing available memory.
- Intelligent Design: Walloc allocator is automatically configured so we dont accidentally grow into the stack memory occupied by our program.

## Aside: Memory in WASM

- WebAssembly linear memory: WebAssembly memories represent a contiguous array of bytes that have a size that is always a multiple of the
  WebAssembly page size (64KiB).
- The Wasm heap is located in its own linear memory space. There are two typical usages of heap: Wasm code calls malloc/free on its own heap.
  The native code of the host calls wasm_runtime_module_malloc to allocate buffer from the Wasm app's heap, and then use it
  for sharing data between host and Wasm.
- The WebAssembly module starts with a certain number of memory pages by default. Emscripten's default is 16 pages. This initial allocation is determined by EMCC or the Rust/WebAssembly compiler toolchain based on the static requirements of the program, with some extra space allocated for the heap.

## Review: Ownership Model

- Independent Reference Counting: Each arena (Render, Scene, Entity) has its own Arc<Mutex<>>, meaning its lifetime is managed independently.
- No Hierarchical Ownership: When a SceneContext is dropped, it doesn't automatically drop the EntityContext objects created from it. Each has its own separate reference count.
- Manual Reset Required: Without nested lifetimes, you need to explicitly call reset_tier() to clear a tier - dropping a SceneContext doesn't automatically reset its arena.

- This approach gives you more flexibility but less automatic cleanup.
  - Each arrow represents an Arc reference, and when all references to an arena are gone, the Arc is dropped, but the memory isn't reclaimed until you explicitly reset the arena.
  - ```
    Scene ------> has reference to ----> Scene Arena
          |
          +--> creates --> Entity A ------> has reference to ----> Entity Arena
          |
          +--> creates --> Entity B ------> has reference to ----> Entity Arena
    ```

## Advanced Considerations

### Graduated Migration

- Some objects might start in the bottom tier but need to be "promoted"
- Implement a mechanism to copy objects to higher tiers when needed

### Size Tuning

- Different scene types will have different optimal arena sizes
- Consider making these configurable or self-adjusting

## Technical Details

- The allocator manages WebAssembly memory pages (64KB chunks) and provides a familiar malloc/free interface. It includes mechanisms for safely transferring data between JavaScript and WebAssembly memory spaces via typed arrays, with built-in bounds checking for memory safety.
- Rust is the perfect language to implement this because of its ownership and scope models help prevent unsafe memory patterns, and a lot of the built in memory functions for Rust are safe wrappers of C instructions.
- This component forms the foundation layer of a 3D rendering engine, enabling optimized memory patterns for graphics data like geometry buffers, textures, and scene graph information, with a focus on supporting LOD (Level of Detail) streaming and efficient memory reuse.

## Technical Specs

- Walloc Allocator

  - Core Walloc is only 1.125MB (Default Only) (18 Pages)

    - This initial memory allocation includes:

      - Compiled Rust code (the Walloc implementation and all other functions)
      - WebAssembly runtime overhead
      - The static data segment (global variables, constant data)
      - Initial stack space
      - The heap area that Walloc will manage

    - Walloc has both a default allocator and a tiered allocator that uses the default as fallback.

      - The default allocator exposes itself to the Web via JS, constructed by wasm-bindgen.

        - Walloc::new() yields a new default allocator, and new_tiered yields the tiered allocator.

          - Memory Layout & Design - Technical Details

            - Layout

              ```
              Render Tier (50%)
                - 128-byte aligned
                - Optimized for GPU access
              Scene Tier (30%)

                - 64-byte aligned
                - Medium lifecycle objects

              Entity Tier (15%)

                - 8-byte aligned
                - Short-lived objects

              Fallback (5%)
                - Traditional allocator
              ```

            - Performance Considerations

              - Allocation in arenas is O(1) using atomic bump allocation
              - Deallocation of entire tiers is O(1)
              - Individual deallocations within arenas are not supported (use contexts instead)
              - Arena-based allocation avoids fragmentation

            - Thread Safety

              - All arena operations use atomic operations for thread safety
              - Mutexes protect concurrent access to arenas
              - Arc enables safe sharing of arenas between contexts

            - Implementation Notes
              - Uses WebAssembly's linear memory model
              - Memory pages are 64KB each
              - The allocator automatically grows memory when needed
              - Proper memory alignment ensures optimal performance for GPU access
