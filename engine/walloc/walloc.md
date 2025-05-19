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

## Technical Details

- The allocator manages WebAssembly memory pages (64KB chunks) and provides a familiar malloc/free interface. It includes mechanisms for safely transferring data between JavaScript and WebAssembly memory spaces via typed arrays, with built-in bounds checking for memory safety.
- Rust is the perfect language to implement this because of its ownership and scope models help prevent unsafe memory patterns, and a lot of the built in memory functions for Rust are safe wrappers of C instructions.
- This component forms the foundation layer of a 3D rendering engine, enabling optimized memory patterns for graphics data like geometry buffers, textures, and scene graph information, with a focus on supporting LOD (Level of Detail) streaming and efficient memory reuse.

## Technical Specs

- Walloc Allocator

  - Walloc's WASM Binary is only 0.024MB
  - Walloc's JS Glue Code is 0.015MB
  - Walloc's Startup Runtime Memory is 1.125MB (Default, Reserved) (~18 Pages)

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

## Caching Considerations

When implementing a producer-consumer system with caching:
Problem: If you use a flag to indicate available memory and write to cache first, subsequent reads may retrieve stale data.
Explanation: In a producer-consumer setup with caching:

- The producer writes data to cache
- The producer sets a flag to true indicating memory is available
- The consumer checks the flag, sees it's true, and reads from cache
- However, if memory was updated directly (bypassing cache), the cache becomes stale

Solution: Always invalidate the cache before setting the availability flag. This ensures that:

- The next read operation will fetch fresh data from memory
- The consumer will always see the most recent updates

This prevents the race condition where cache contains outdated information while the flag indicates data is ready.
This technique is called "polling" or "scheduled polling" and is common in page based memory allocators.

## Advanced Considerations - For Frequent Allocations

### Vectorization and SIMD

Since you're in a WebAssembly context, you can use SIMD (Single Instruction, Multiple Data) instructions to process multiple bytes at once:

```rust
// Import WASM SIMD intrinsics
use core::arch::wasm32::*;

// Example: Fill memory with a value using v128 operations (16 bytes at once)
pub fn fast_fill(ptr: *mut u8, size: usize, value: u8) {
    let aligned_size = size & !15; // Round down to multiple of 16
    let simd_value = v128_set_splat_i8(value as i8);

    // Process 16 bytes at a time
    for i in (0..aligned_size).step_by(16) {
        unsafe {
            let dest = ptr.add(i) as *mut v128;
            v128_store(dest, simd_value);
        }
    }

    // Handle remaining bytes
    for i in aligned_size..size {
        unsafe {
            *ptr.add(i) = value;
        }
    }
}
```

### Type Punning for Wider Access

```rust
pub fn fast_copy_u32(src: *const u8, dst: *mut u8, count_bytes: usize) {
    let count_u32 = count_bytes / 4;

    // Reinterpret as u32 pointers
    let src_u32 = src as *const u32;
    let dst_u32 = dst as *mut u32;

    // Copy 4 bytes at a time
    for i in 0..count_u32 {
        unsafe {
            *dst_u32.add(i) = *src_u32.add(i);
        }
    }

    // Handle remaining bytes
    for i in (count_u32 * 4)..count_bytes {
        unsafe {
            *dst.add(i) = *src.add(i);
        }
    }
}
```

### Alignment Operations

Ensuring your memory operations are aligned to cache line boundaries (64 bytes) can significantly improve performance:

```rust
pub fn aligned_copy(src: *const u8, dst: *mut u8, size: usize) {
    // Check if pointers are aligned to cache line (64 bytes)
    if (src as usize % 64 == 0) && (dst as usize % 64 == 0) && (size % 64 == 0) {
        // Fast path: 64-byte aligned copy
        for i in (0..size).step_by(64) {
            // Copy an entire cache line at once
            unsafe {
                let src_ptr = src.add(i) as *const [u8; 64];
                let dst_ptr = dst.add(i) as *mut [u8; 64];
                *dst_ptr = *src_ptr;
            }
        }
    } else {
        // Fallback for unaligned memory
        for i in 0..size {
            unsafe {
                *dst.add(i) = *src.add(i);
            }
        }
    }
}
```
