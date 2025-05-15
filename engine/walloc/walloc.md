# SUBMODULE_001: WALLOC: A WebAssembly memory allocator using Rust

Walloc is a custom memory allocator implemented in Rust for WebAssembly applications, specifically optimized for 3D graphics rendering engines. It provides efficient memory management with direct control over the WASM linear memory space, enabling high-performance memory utilization within browser environments.

## Technical Specs

- Default Walloc Allocator

  - Walloc::new()

    - Core Walloc is only 1.125MB

      - This initial memory allocation includes:

        - Compiled Rust code (the Walloc implementation and all other functions)
        - WebAssembly runtime overhead
        - The static data segment (global variables, constant data)
        - Initial stack space
        - The heap area that the custom DefaultAllocator will manage

      - The WebAssembly module starts with a certain number of memory pages by default, and 18 pages (1.125MB) is quite typical. This initial allocation is determined by the Rust/WebAssembly compiler toolchain based on the static requirements of your program, with some extra space allocated for the heap.

## Aside: Memory in WASM

- WebAssembly linear memory: WebAssembly memories represent a contiguous array of bytes that have a size that is always a multiple of the
  WebAssembly page size (64KiB).
- The Wasm heap is located in its own linear memory space. There are two typical usages of heap: Wasm code calls malloc/free on its own heap.
  The native code of the host calls wasm_runtime_module_malloc to allocate buffer from the Wasm app's heap, and then use it
  for sharing data between host and Wasm.

## Key Features

- Efficient Memory Utilization: Intelligently manages WebAssembly's linear memory, gradually growing as needed up to near the full 4GB address space.
- Direct Memory Access: Provides low-level memory manipulation with typed array views, supporting raw memory operations critical for graphics applications.
- Configurable Allocation Strategy: Uses a first-fit allocation strategy with block splitting and coalescing to minimize fragmentation.
- Memory Lifecycle Management: Supports allocation, deallocation, reallocation, and complete memory reset functionality.
- Browser Compatibility: Designed to work around browser-specific memory limitations while maximizing available memory.
- Intelligent Design: Walloc allocator is automatically configured so we dont accidentally grow into the stack memory occupied by our program.

## Technical Details

- The allocator manages WebAssembly memory pages (64KB chunks) and provides a familiar malloc/free interface. It includes mechanisms for safely transferring data between JavaScript and WebAssembly memory spaces via typed arrays, with built-in bounds checking for memory safety.
- Rust is the perfect language to implement this in because of its ownership and scope models.
- This component forms the foundation layer of a 3D rendering engine, enabling optimized memory patterns for graphics data like geometry buffers, textures, and scene graph information, with a focus on supporting LOD (Level of Detail) streaming and efficient memory reuse.

### Research & Mockup

- Emscripten and wasm_bindgen both have access to allocation methods internally and as handles, so investigate there...
- Use a Tiered Arena Allocator. (think Minecraft chunks)

A tiered arena approach is absolutely possible and would be an excellent fit for your WebAssembly game engine. This hierarchical memory management strategy aligns perfectly with the natural hierarchy in a game scene.
Tiered Arena Architecture
The tiered approach you described can be structured like this:

Top Tier: Scene Mesh Data

Largest memory block
Contains vertex data, indices, textures, materials
Longest-lived arena, reset only when the entire scene changes

Middle Tier: Scene Logic

Contains gameplay systems, AI behavior trees, event handlers
Medium lifecycle - might be reset when gameplay systems change
Smaller than mesh data but larger than objects

Bottom Tier: Scene Objects

Contains dynamic entities, particles, temporary effects
Shortest lifecycle - could be reset frequently
Potentially the smallest arena size but highest allocation frequency

Advanced Considerations

Nested Arena Lifetimes

You could design the system so higher tiers "own" lower tiers
When a higher tier is reset, it automatically resets all lower tiers

Cross-Arena References

References between arenas need careful management to prevent dangling pointers
Consider using generational indices instead of direct references

Graduated Migration

Some objects might start in the bottom tier but need to be "promoted"
Implement a mechanism to copy objects to higher tiers when needed

Size Tuning

Different scene types will have different optimal arena sizes
Consider making these configurable or self-adjusting

WASM-Specific Optimizations

Align arenas with WASM page boundaries (64KiB)
Implement memory growth strategies that efficiently request new pages

Psedo Implementation in Rust
Here's how you could implement this tiered system in Rust:

```rust
struct TieredArena {
    mesh_arena: Arena,
    logic_arena: Arena,
    object_arena: Arena,
}

impl TieredArena {
    fn new(mesh_size: usize, logic_size: usize, object_size: usize) -> Self {
        TieredArena {
            mesh_arena: Arena::new(mesh_size),
            logic_arena: Arena::new(logic_size),
            object_arena: Arena::new(object_size),
        }
    }

    // Allocate mesh data in the top tier
    fn alloc_mesh<T>(&mut self, value: T) -> &mut T {
        self.mesh_arena.alloc(value)
    }

    // Allocate logic components in the middle tier
    fn alloc_logic<T>(&mut self, value: T) -> &mut T {
        self.logic_arena.alloc(value)
    }

    // Allocate dynamic objects in the bottom tier
    fn alloc_object<T>(&mut self, value: T) -> &mut T {
        self.object_arena.alloc(value)
    }

    // Reset just the object arena (most frequent)
    fn reset_objects(&mut self) {
        self.object_arena.reset();
    }

    // Reset logic and objects (less frequent)
    fn reset_logic(&mut self) {
        self.logic_arena.reset();
        self.object_arena.reset();
    }

    // Reset everything (least frequent)
    fn reset_all(&mut self) {
        self.mesh_arena.reset();
        self.logic_arena.reset();
        self.object_arena.reset();
    }

}
```
