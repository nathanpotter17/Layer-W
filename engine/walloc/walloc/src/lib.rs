use wasm_bindgen::prelude::*;
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};

// For Loading Asset Data
use reqwest::Client;
use wasm_bindgen_futures::JsFuture;
use js_sys::{Promise, Uint8Array, Array};

#[wasm_bindgen]
pub struct Walloc {
    strategy: AllocatorStrategy,
    memory_base: *mut u8,
    memory_size: usize,
}

pub enum AllocatorStrategy {
    Default(DefaultAllocator),
    Tiered(TieredAllocator),
}

#[repr(C)]
struct BlockHeader {
    size: usize,
    next: *mut BlockHeader,
    is_free: bool,
    tier: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    Render = 0,   // Top tier: Mesh data, render targets (frequent reallocation, cache-aligned)
    Scene = 1,    // Middle tier: Scene data, gameplay systems (medium lifecycle)
    Entity = 2,   // Bottom tier: Actors, particles, effects (short lifecycle)
}

impl Tier {
    fn from_u8(value: u8) -> Option<Tier> {
        match value {
            0 => Some(Tier::Render),
            1 => Some(Tier::Scene),
            2 => Some(Tier::Entity),
            _ => None,
        }
    }
}

pub struct DefaultAllocator {
    free_list_head: *mut BlockHeader,
    heap_start: *mut u8,
    heap_end: *mut u8,
}

pub struct Arena {
    base: *mut u8,
    size: usize,
    current_offset: AtomicUsize,
    tier: Tier,

    high_water_mark: AtomicUsize,  // Track the highest allocation point
    total_allocated: AtomicUsize,  // Track total bytes allocated, even when recycled
}

pub struct MemoryOwner {
    // The arena this entity belongs to
    arena: Arc<Mutex<Arena>>,
    // Memory regions this entity owns (offset, size)
    allocations: Vec<(usize, usize)>,
}

pub struct TieredAllocator {
    render_arena: Arc<Mutex<Arena>>,
    scene_arena: Arc<Mutex<Arena>>,
    entity_arena: Arc<Mutex<Arena>>,
}

impl DefaultAllocator {
    pub fn new(heap_start: *mut u8, heap_size: usize) -> Self {
        let heap_end = unsafe { heap_start.add(heap_size) };
        
        // Initialize with a single free block covering the entire heap
        let initial_block = heap_start as *mut BlockHeader;
        unsafe {
            (*initial_block).size = heap_size;
            (*initial_block).next = std::ptr::null_mut();
            (*initial_block).is_free = true;
            (*initial_block).tier = 255; // Default tier (not in any arena)
        }
        
        DefaultAllocator {
            free_list_head: initial_block,
            heap_start,
            heap_end,
        }
    }
    
    pub fn malloc(&mut self, size: usize) -> *mut u8 {
        // Align the requested size to 8 bytes (common alignment requirement)
        let aligned_size;
        if size > 1024 {
            aligned_size = (size + 63) & !63;  // Cache Line Alignment, for larger blocks.
        } else {
            aligned_size = (size + 7) & !7;    // Regular 8 byte alignment for smaller blocks.
        }
        let total_size = aligned_size + std::mem::size_of::<BlockHeader>();
        
        // Find a suitable free block using first-fit strategy
        let mut current = self.free_list_head;
        let mut prev: *mut BlockHeader = std::ptr::null_mut();
        
        while !current.is_null() {
            unsafe {
                if (*current).is_free && (*current).size >= total_size {
                    // Found a block that's big enough
                    
                    // Check if we should split this block
                    if (*current).size >= total_size + std::mem::size_of::<BlockHeader>() + 8 {
                        // Block is large enough to split
                        let new_block = (current as *mut u8).add(total_size) as *mut BlockHeader;
                        (*new_block).size = (*current).size - total_size;
                        (*new_block).next = (*current).next;
                        (*new_block).is_free = true;
                        (*new_block).tier = (*current).tier;
                        
                        // Update current block
                        (*current).size = total_size;
                        (*current).next = new_block;
                    }
                    
                    // Mark the block as used
                    (*current).is_free = false;
                    
                    // Return pointer to the data area (after the header)
                    return (current as *mut u8).add(std::mem::size_of::<BlockHeader>());
                }
                
                // Move to the next block
                prev = current;
                current = (*current).next;
            }
        }
        
        // If we get here, no suitable block was found.
        // We could try to grow the heap if supported
        self.grow_heap(total_size)
    }
    
    pub fn free(&mut self, ptr: *mut u8) {
        if ptr.is_null() {
            return; // Null pointer, do nothing
        }
        
        // Get the block header
        let block = unsafe { (ptr as *mut u8).sub(std::mem::size_of::<BlockHeader>()) } as *mut BlockHeader;
        
        unsafe {
            // Mark the block as free
            (*block).is_free = true;
            
            // Try to coalesce with the next block if it's free
            let next_block = (*block).next;
            if !next_block.is_null() && (*next_block).is_free {
                (*block).size += (*next_block).size;
                (*block).next = (*next_block).next;
            }
            
            // Try to find the previous block to coalesce if it's free
            let mut current = self.free_list_head;
            while !current.is_null() && (*current).next != block {
                current = (*current).next;
            }
            
            if !current.is_null() && (*current).is_free {
                // Previous block is free, coalesce
                (*current).size += (*block).size;
                (*current).next = (*block).next;
            }
        }
    }
    
    // Helper function to grow the heap when needed
    fn grow_heap(&mut self, size_needed: usize) -> *mut u8 {
        // Calculate how many WebAssembly pages we need (64KiB per page)
        let pages_needed = (size_needed + 65535) / 65536;
        
        // Try to grow memory
        let old_pages = core::arch::wasm32::memory_grow(0, pages_needed);
        if old_pages == usize::MAX {
            return std::ptr::null_mut(); // Failed to grow memory
        }
        
        // We successfully grew the memory
        let new_block_start = self.heap_end;
        let new_block_size = pages_needed * 65536;
        
        // Update heap end
        self.heap_end = unsafe { self.heap_end.add(new_block_size) };
        
        // Initialize the new block
        let new_block = new_block_start as *mut BlockHeader;
        unsafe {
            (*new_block).size = new_block_size;
            (*new_block).next = std::ptr::null_mut();
            (*new_block).is_free = true;
            (*new_block).tier = 255; // Default tier (not in any arena)
        }
        
        // Add the new block to our free list
        // Find the last block in our current free list
        let mut current = self.free_list_head;
        while unsafe { !current.is_null() && !(*current).next.is_null() } {
            current = unsafe { (*current).next };
        }
        
        if !current.is_null() {
            unsafe { (*current).next = new_block; }
        } else {
            self.free_list_head = new_block;
        }
        
        // Now try to allocate again with our newly expanded heap
        self.malloc(size_needed - std::mem::size_of::<BlockHeader>())
    }

    pub fn is_ptr_in_heap(&self, ptr: *mut u8) -> bool {
        ptr >= self.heap_start && ptr < self.heap_end
    }
}

// Arena implementation for tiered allocation
impl Arena {
    pub fn new(base: *mut u8, size: usize, tier: Tier) -> Self {
        Self {
            base,
            size,
            current_offset: AtomicUsize::new(0),
            tier,
            high_water_mark: AtomicUsize::new(0),
            total_allocated: AtomicUsize::new(0),
        }
    }
    
    // Bump allocation - very fast track total allocated memory and high water mark
    pub fn allocate(&self, size: usize) -> Option<(*mut u8, usize)> {
        // Align size to appropriate boundary based on tier
        let aligned_size = match self.tier {
            Tier::Render => (size + 127) & !127,  // 128-byte alignment for GPU warp access
            Tier::Scene => (size + 63) & !63,     // 64-byte alignment for cache lines
            Tier::Entity => (size + 7) & !7,      // 8-byte alignment for other tiers
        };
        
        // Atomic compare-and-swap to reserve space
        let mut current_offset = self.current_offset.load(Ordering::Relaxed);
        loop {
            // Check if we have enough space
            if current_offset + aligned_size > self.size {
                return None; // Not enough space
            }
            
            // Try to advance the offset
            let new_offset = current_offset + aligned_size;
            match self.current_offset.compare_exchange(
                current_offset, 
                new_offset,
                Ordering::SeqCst,
                Ordering::Relaxed
            ) {
                Ok(_) => {
                    // Success! Update the high water mark if needed
                    let mut hwm = self.high_water_mark.load(Ordering::Relaxed);
                    if new_offset > hwm {
                        self.high_water_mark.store(new_offset, Ordering::Relaxed);
                    }
                    
                    // Update total allocated bytes
                    self.total_allocated.fetch_add(aligned_size, Ordering::Relaxed);
                    
                    // Return pointer to the allocated memory
                    let ptr = unsafe { self.base.add(current_offset) };
                    return Some((ptr, aligned_size));
                }
                Err(actual) => {
                    // Try again with the updated offset
                    current_offset = actual;
                }
            }
        }
    }
    
    // Reset the entire arena - very efficient way to free everything at once
    pub fn reset(&self) {
        self.current_offset.store(0, Ordering::SeqCst);
    }
    
    // Check if a pointer belongs to this arena
    pub fn contains(&self, ptr: *mut u8) -> bool {
        let end = unsafe { self.base.add(self.size) };
        ptr >= self.base && ptr < end
    }
    
    // Get current usage
    pub fn usage(&self) -> usize {
        self.current_offset.load(Ordering::Relaxed)
    }
    
    // Get capacity
    pub fn capacity(&self) -> usize {
        self.size
    }

    // Fast compact operation that preserves the first 'preserve_bytes' of memory
    // Note: This will return false if preserve_bytes > current_offset.
    // The TieredAllocator::fast_compact_tier handles the case of growing
    // memory when needed before calling this method.
    pub fn fast_compact(&self, preserve_bytes: usize) -> bool {
        // Ensure we don't preserve more than our current offset
        let current = self.current_offset.load(Ordering::Relaxed);
        if preserve_bytes > current {
            return false; // Can't preserve more than we've allocated
        }
        
        // Simple atomic store to update the allocation pointer
        // This effectively "recycles" all memory after the preserved section
        self.current_offset.store(preserve_bytes, Ordering::SeqCst);
        
        true
    }

    pub fn get_stats(&self) -> (usize, usize, usize, usize) {
        (
            self.usage(),
            self.capacity(),
            self.high_water_mark.load(Ordering::Relaxed),
            self.total_allocated.load(Ordering::Relaxed)
        )
    }
}

// TieredAllocator implementation
impl TieredAllocator {
    pub fn new(memory_base: *mut u8, memory_size: usize) -> Self {
        // Calculate sizes for each arena
        // Render tier: 50% of memory, Scene tier: 30%, Entity tier: 20%
        let render_size = (memory_size * 50) / 100;
        let scene_size = (memory_size * 30) / 100;
        let entity_size = (memory_size * 20) / 100;
        
        // Create arenas
        let render_base = memory_base;
        let scene_base = unsafe { render_base.add(render_size) };
        let entity_base = unsafe { scene_base.add(scene_size) };
        
        let render_arena = Arena::new(render_base, render_size, Tier::Render);
        let scene_arena = Arena::new(scene_base, scene_size, Tier::Scene);
        let entity_arena = Arena::new(entity_base, entity_size, Tier::Entity);
        
        TieredAllocator {
            render_arena: Arc::new(Mutex::new(render_arena)),
            scene_arena: Arc::new(Mutex::new(scene_arena)),
            entity_arena: Arc::new(Mutex::new(entity_arena)),
        }
    }

    // Fast compact for a specific tier with intelligent growing
    pub fn fast_compact_tier(&mut self, tier: Tier, preserve_bytes: usize) -> bool {
        // Get current allocation and capacity for the specified tier
        let (current_offset, capacity) = match tier {
            Tier::Render => {
                if let Ok(arena) = self.render_arena.lock() {
                    (arena.current_offset.load(Ordering::Relaxed), arena.capacity())
                } else {
                    return false;
                }
            },
            Tier::Scene => {
                if let Ok(arena) = self.scene_arena.lock() {
                    (arena.current_offset.load(Ordering::Relaxed), arena.capacity())
                } else {
                    return false;
                }
            },
            Tier::Entity => {
                if let Ok(arena) = self.entity_arena.lock() {
                    (arena.current_offset.load(Ordering::Relaxed), arena.capacity())
                } else {
                    return false;
                }
            },
        };
        
        // If we need more space than currently allocated
        if preserve_bytes > current_offset {
            // Check if the requested size exceeds our capacity
            if preserve_bytes > capacity {
                // We need to grow the heap, but first check if it's feasible
                
                // Get total WebAssembly memory size (can't exceed 4GB in wasm32)
                let total_current_pages = core::arch::wasm32::memory_size(0);
                let max_pages = 65536; // Max 4GB (65536 pages * 64KB per page)
                
                // Calculate how many more pages we need
                let additional_bytes_needed = preserve_bytes - current_offset;
                let additional_pages_needed = (additional_bytes_needed + 65535) / 65536;
                
                // Check if growing would exceed the 4GB limit
                if total_current_pages + additional_pages_needed > max_pages {
                    #[cfg(target_arch = "wasm32")]
                    {
                        use web_sys::console;
                        web_sys::console::log_1(&format!(
                            "Cannot grow memory - would exceed 4GB limit. Current pages: {}, needed: {}, max: {}",
                            total_current_pages, additional_pages_needed, max_pages
                        ).into());
                    }
                    return false;
                }
                
                // Try to grow the heap
                #[cfg(target_arch = "wasm32")]
                {   
                    use web_sys::console;
                    web_sys::console::log_1(&format!(
                        "Growing heap for tier {:?} compact - current: {}, preserve: {}, growing by: {} pages",
                        tier, current_offset, preserve_bytes, additional_pages_needed
                    ).into());
                }
                
                // Create temporary storage to hold data we want to preserve
                let preserve_data = if current_offset > 0 {
                    // Get a reference to the arena to copy data from
                    let arena_ref = match tier {
                        Tier::Render => self.render_arena.clone(),
                        Tier::Scene => self.scene_arena.clone(),
                        Tier::Entity => self.entity_arena.clone(),
                    };
                    
                    // Copy the data we want to preserve
                    if let Ok(arena) = arena_ref.lock() {
                        // Only copy what's currently allocated (not what we'll grow to)
                        let bytes_to_copy = current_offset.min(preserve_bytes);
                        let mut data = Vec::with_capacity(bytes_to_copy);
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                arena.base,
                                data.as_mut_ptr(),
                                bytes_to_copy
                            );
                            data.set_len(bytes_to_copy);
                        }
                        Some(data)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Grow the heap
                let new_mem = self.grow_heap(additional_bytes_needed, tier);
                if new_mem.is_null() {
                    #[cfg(target_arch = "wasm32")]
                    {   
                        use web_sys::console;
                        web_sys::console::log_1(&JsValue::from_str("Failed to grow memory for compact operation"));
                    }
                    return false;
                }
                
                // Copy preserved data to the new arena if needed
                if let Some(data) = preserve_data {
                    match tier {
                        Tier::Render => {
                            if let Ok(arena) = self.render_arena.lock() {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(
                                        data.as_ptr(),
                                        arena.base,
                                        data.len()
                                    );
                                }
                                // Set the current offset to include our preserved data
                                arena.current_offset.store(data.len(), Ordering::SeqCst);
                            }
                        },
                        Tier::Scene => {
                            if let Ok(arena) = self.scene_arena.lock() {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(
                                        data.as_ptr(),
                                        arena.base,
                                        data.len()
                                    );
                                }
                                arena.current_offset.store(data.len(), Ordering::SeqCst);
                            }
                        },
                        Tier::Entity => {
                            if let Ok(arena) = self.entity_arena.lock() {
                                unsafe {
                                    std::ptr::copy_nonoverlapping(
                                        data.as_ptr(),
                                        arena.base,
                                        data.len()
                                    );
                                }
                                arena.current_offset.store(data.len(), Ordering::SeqCst);
                            }
                        },
                    }
                }
                
                // Now ensure the offset is correctly set to preserve_bytes
                match tier {
                    Tier::Render => {
                        if let Ok(arena) = self.render_arena.lock() {
                            arena.current_offset.store(preserve_bytes, Ordering::SeqCst);
                        }
                    },
                    Tier::Scene => {
                        if let Ok(arena) = self.scene_arena.lock() {
                            arena.current_offset.store(preserve_bytes, Ordering::SeqCst);
                        }
                    },
                    Tier::Entity => {
                        if let Ok(arena) = self.entity_arena.lock() {
                            arena.current_offset.store(preserve_bytes, Ordering::SeqCst);
                        }
                    },
                }
                
                return true; // Successfully grew and preserved
            } else {
                // We have enough capacity, just need to allocate up to preserve_bytes
                match tier {
                    Tier::Render => {
                        if let Ok(arena) = self.render_arena.lock() {
                            // Set the current offset to preserve_bytes
                            arena.current_offset.store(preserve_bytes, Ordering::SeqCst);
                            return true;
                        }
                    },
                    Tier::Scene => {
                        if let Ok(arena) = self.scene_arena.lock() {
                            arena.current_offset.store(preserve_bytes, Ordering::SeqCst);
                            return true;
                        }
                    },
                    Tier::Entity => {
                        if let Ok(arena) = self.entity_arena.lock() {
                            arena.current_offset.store(preserve_bytes, Ordering::SeqCst);
                            return true;
                        }
                    },
                }
            }
        } else {
            // Current allocation is sufficient, proceed with normal compact
            match tier {
                Tier::Render => {
                    if let Ok(arena) = self.render_arena.lock() {
                        return arena.fast_compact(preserve_bytes);
                    }
                },
                Tier::Scene => {
                    if let Ok(arena) = self.scene_arena.lock() {
                        return arena.fast_compact(preserve_bytes);
                    }
                },
                Tier::Entity => {
                    if let Ok(arena) = self.entity_arena.lock() {
                        return arena.fast_compact(preserve_bytes);
                    }
                },
            }
        }
        
        false
    }

    // Grow heap for a specific tier - exact allocation, no overhead
    pub fn grow_heap(&mut self, size_needed: usize, tier: Tier) -> *mut u8 {
        // Calculate how many WebAssembly pages we need (64KiB per page)
        let pages_needed = (size_needed + 65535) / 65536;
        
        // Try to grow memory
        let old_pages = core::arch::wasm32::memory_grow(0, pages_needed);
        if old_pages == usize::MAX {
            // Failed to grow memory - log failure
            return std::ptr::null_mut();
        }
        
        // We successfully grew the memory
        let new_block_size = pages_needed * 65536;
        
        // Calculate the base address for the new memory
        let new_memory_base = unsafe { 
            (old_pages * 65536) as *mut u8 
        };
        
        // Create a new arena for the specific tier
        let new_arena = Arena::new(new_memory_base, new_block_size, tier);
        
        // Based on the tier, update or replace the corresponding arena
        match tier {
            Tier::Render => {
                if let Ok(mut old_arena) = self.render_arena.lock() {
                    *old_arena = new_arena;
                }
            },
            Tier::Scene => {
                if let Ok(mut old_arena) = self.scene_arena.lock() {
                    *old_arena = new_arena;
                }
            },
            Tier::Entity => {
                if let Ok(mut old_arena) = self.entity_arena.lock() {
                    *old_arena = new_arena;
                }
            },
        }
        
        // Return a non-null pointer to indicate success
        // The actual allocation will happen in the caller
        new_memory_base
    }
    
    pub fn allocate_with_owner(&mut self, size: usize, tier: Tier) -> Option<(MemoryOwner, *mut u8)> {
        let arena = match tier {
            Tier::Render => &self.render_arena,
            Tier::Scene => &self.scene_arena,
            Tier::Entity => &self.entity_arena,
        };
        
        // Try to allocate from the selected arena
        if let Ok(arena_lock) = arena.lock() {
            if let Some((ptr, alloc_size)) = arena_lock.allocate(size) {
                // Create a memory owner for this allocation
                let offset = (ptr as usize) - (arena_lock.base as usize);
                let owner = MemoryOwner {
                    arena: Arc::clone(arena),
                    allocations: vec![(offset, alloc_size)],
                };
                
                return Some((owner, ptr));
            }
        }
        
        // If the arena allocation failed, try to grow the heap
        // First grow the heap
        let ptr = self.grow_heap(size, tier);
        
        // If growth failed, return None
        if ptr.is_null() {
            return None;
        }
        
        // Try allocation again with the newly expanded arena
        let arena = match tier {
            Tier::Render => &self.render_arena,
            Tier::Scene => &self.scene_arena,
            Tier::Entity => &self.entity_arena,
        };
        
        // Try to allocate from the selected arena after growing
        if let Ok(arena_lock) = arena.lock() {
            if let Some((new_ptr, alloc_size)) = arena_lock.allocate(size) {
                // Create a memory owner for this allocation
                let offset = (new_ptr as usize) - (arena_lock.base as usize);
                let owner = MemoryOwner {
                    arena: Arc::clone(arena),
                    allocations: vec![(offset, alloc_size)],
                };
                
                return Some((owner, new_ptr));
            }
        }
        
        // If allocation still fails after growing, return None, we're out of memory.
        None
    }
    
    pub fn allocate(&mut self, size: usize, tier: Tier) -> *mut u8 {
        // First attempt: try to allocate from the selected arena
        let arena = match tier {
            Tier::Render => &self.render_arena,
            Tier::Scene => &self.scene_arena,
            Tier::Entity => &self.entity_arena,
        };
        
        if let Ok(arena_lock) = arena.lock() {
            if let Some((ptr, _)) = arena_lock.allocate(size) {
                return ptr; // Allocation succeeded
            }
        }
        
        // First attempt failed - try to grow the heap
        let ptr = self.grow_heap(size, tier);
        
        // If growth succeeded, try allocation again
        if !ptr.is_null() {
            let arena = match tier {
                Tier::Render => &self.render_arena,
                Tier::Scene => &self.scene_arena,
                Tier::Entity => &self.entity_arena,
            };
            
            if let Ok(arena_lock) = arena.lock() {
                if let Some((new_ptr, _)) = arena_lock.allocate(size) {
                    return new_ptr;
                }
            }
        } else {
            // Growth failed - try recycling and then allocating
            
            // Get current stats for this tier to determine how much we're using
            let (current_usage, _, _, _) = match tier {
                Tier::Render => {
                    if let Ok(arena) = self.render_arena.lock() {
                        arena.get_stats()
                    } else {
                        (0, 0, 0, 0)
                    }
                },
                Tier::Scene => {
                    if let Ok(arena) = self.scene_arena.lock() {
                        arena.get_stats()
                    } else {
                        (0, 0, 0, 0)
                    }
                },
                Tier::Entity => {
                    if let Ok(arena) = self.entity_arena.lock() {
                        arena.get_stats()
                    } else {
                        (0, 0, 0, 0)
                    }
                },
            };
            
            // If we're using enough memory that recycling might help
            if current_usage > size {
                web_sys::console::log_1(&format!(
                    "Growth failed, attempting to reset tier {:?} completely to make space",
                    tier
                ).into());
                
                // Reset this tier completely - clearer than preserving 0 bytes
                self.reset_tier(tier);
                
                // Try allocation again after resetting
                let arena = match tier {
                    Tier::Render => &self.render_arena,
                    Tier::Scene => &self.scene_arena,
                    Tier::Entity => &self.entity_arena,
                };
                
                if let Ok(arena_lock) = arena.lock() {
                    if let Some((new_ptr, _)) = arena_lock.allocate(size) {
                        return new_ptr; // Allocation succeeded after resetting
                    }
                }
            }
        }
        
        // If all attempts fail, return null
        std::ptr::null_mut()
    }
    
    // Check if pointer is in any arena
    fn is_ptr_in_arena(&self, ptr: *mut u8) -> bool {
        if let Ok(arena) = self.render_arena.lock() {
            if arena.contains(ptr) {
                return true;
            }
        }
        
        if let Ok(arena) = self.scene_arena.lock() {
            if arena.contains(ptr) {
                return true;
            }
        }
        
        if let Ok(arena) = self.entity_arena.lock() {
            if arena.contains(ptr) {
                return true;
            }
        }
        
        false
    }
    
    // Reset a specific tier
    pub fn reset_tier(&mut self, tier: Tier) {
        match tier {
            Tier::Render => {
                if let Ok(arena) = self.render_arena.lock() {
                    arena.reset();
                }
            },
            Tier::Scene => {
                if let Ok(arena) = self.scene_arena.lock() {
                    arena.reset();
                }
            },
            Tier::Entity => {
                if let Ok(arena) = self.entity_arena.lock() {
                    arena.reset();
                }
            },
        }
    }
    
    pub fn tier_stats(&self, tier: Tier) -> (usize, usize, usize, usize) {
        match tier {
            Tier::Render => {
                if let Ok(arena) = self.render_arena.lock() {
                    arena.get_stats()
                } else {
                    (0, 0, 0, 0)
                }
            },
            Tier::Scene => {
                if let Ok(arena) = self.scene_arena.lock() {
                    arena.get_stats()
                } else {
                    (0, 0, 0, 0)
                }
            },
            Tier::Entity => {
                if let Ok(arena) = self.entity_arena.lock() {
                    arena.get_stats()
                } else {
                    (0, 0, 0, 0)
                }
            },
        }
    }
    
    // Check if a pointer is valid
    pub fn is_ptr_valid(&self, ptr: *mut u8) -> bool {
        self.is_ptr_in_arena(ptr)
    }
}

pub enum AssetType {
    Image = 0,
    Json = 1,
}

struct AssetMetadata {
    asset_type: AssetType,
    size: usize,
    offset: usize,
}

pub struct AssetManager {
    allocator: Walloc<TieredAllocator>,
    http_client: Client,
    assets: Arc<Mutex<HashMap<String, AssetMetadata>>>,
    base_url: String,
}

impl AssetManager {
    pub fn new() -> Self {
        let http_client = Client::new();
        let t_alloc = Walloc::new_tiered();

        AssetManager {
            t_alloc,
            http_client,
            assets: Arc::new(Mutex::new(HashMap::new()))
            base_url: base_url.unwrap_or_else(|| "".to_string()),
        }
    }

    async fn print_json() -> Result<(), Box<dyn std::error::Error>> {
        let resp = self.http_client.get("https://jsonplaceholder.typicode.com/todos/1")
            .await?
            .json::<HashMap<String, String>>()
            .await?;
        println!("{resp:#?}");
        Ok(())
    }

    async fn load_asset(&self, url: String, asset_type: u8) -> Result<usize, JsValue> {
         let asset_type = match asset_type {
            0 => AssetType::Image,
            1 => AssetType::Json,
            _ => return Err(JsValue::from_str("Invalid asset type: must be 0 (Image) or 1 (Json)")),
        };
        
        // Fetch the asset using reqwest
        let response = self.http_client.get(&full_url)
            .send()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch: {}", e)))?;
            
        let bytes = response.bytes()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get bytes: {}", e)))?;
            
        let data_size = bytes.len();
        println!(&bytes.into())
        println!(&data_size.into())

        // To store in WASM memory: copy_to_js, allocate_tiered, or something custom?
    }
}

#[wasm_bindgen]
impl Walloc {
    pub fn new() -> Self {
        let memory_base = core::arch::wasm32::memory_size(0) as *mut u8;
        let memory_size = (core::arch::wasm32::memory_size(0) * 65536) as usize;
        
        // Use DefaultAllocator by default
        let strategy = AllocatorStrategy::Default(
            DefaultAllocator::new(memory_base, memory_size)
        );
        
        Walloc {
            strategy,
            memory_base,
            memory_size,
        }
    }
    
    // Create a new Walloc with TieredAllocator
    #[wasm_bindgen]
    pub fn new_tiered() -> Self {
        let memory_base = core::arch::wasm32::memory_size(0) as *mut u8;
        let memory_size = (core::arch::wasm32::memory_size(0) * 65536) as usize;
        
        // Use TieredAllocator
        let strategy = AllocatorStrategy::Tiered(
            TieredAllocator::new(memory_base, memory_size)
        );
        
        Walloc {
            strategy,
            memory_base,
            memory_size,
        }
    }
    
    // Get a direct view into WASM memory as a typed array
    #[wasm_bindgen]
    pub fn get_memory_view(&self, offset: usize, length: usize) -> Result<js_sys::Uint8Array, JsValue> {
        if offset + length > self.memory_size {
            return Err(JsValue::from_str("Memory access out of bounds"));
        }
        
        unsafe {
            let ptr = self.memory_base.add(offset);
            let mem_slice = std::slice::from_raw_parts(ptr, length);
            Ok(js_sys::Uint8Array::from(mem_slice))
        }
    }
    
    // Allocate memory from a specific tier
    #[wasm_bindgen]
    pub fn allocate_tiered(&mut self, size: usize, tier_number: u8) -> usize {
        let tier = match Tier::from_u8(tier_number) {
            Some(t) => t,
            None => Tier::Entity, // Default to Entity tier if invalid
        };

        let ptr = match &mut self.strategy {
            AllocatorStrategy::Tiered(allocator) => {
                allocator.allocate(size, tier)
            },
            _ => {
                // Return 0 for non-tiered allocators
                return 0;
            }
        };

        self.memory_size = core::arch::wasm32::memory_size(0) * 65536;
        
        // Return offset from memory base
        if ptr.is_null() {
            0 // Error case, return 0 (null) pointer
        } else {
            (ptr as usize) - (self.memory_base as usize)
        }
    }

    #[wasm_bindgen]
    pub fn fast_compact_tier(&mut self, tier_number: u8, preserve_bytes: usize) -> bool {
        let tier = match Tier::from_u8(tier_number) {
            Some(t) => t,
            None => return false,
        };
        
        match &mut self.strategy {
            AllocatorStrategy::Tiered(allocator) => {
                allocator.fast_compact_tier(tier, preserve_bytes)
            },
            _ => false,
        }
    }
    
    // Reset a specific tier
    #[wasm_bindgen]
    pub fn reset_tier(&mut self, tier_number: u8) -> bool {
        let tier = match Tier::from_u8(tier_number) {
            Some(t) => t,
            None => return false,
        };
        
        match &mut self.strategy {
            AllocatorStrategy::Tiered(allocator) => {
                allocator.reset_tier(tier);
                true
            },
            _ => false,
        }
    }
    
    // Standard allocate (backward compatibility)
    #[wasm_bindgen]
    pub fn allocate(&mut self, size: usize) -> usize {
        // For backward compatibility, use regular malloc in default mode
        let ptr = match &mut self.strategy {
            AllocatorStrategy::Default(allocator) => {
                allocator.malloc(size)
            },
            _ => {
                return 0;
            }
        };

        self.memory_size = core::arch::wasm32::memory_size(0) * 65536;
        
        // Return offset from memory base
        if ptr.is_null() {
            0 // Error case, return 0 (null) pointer
        } else {
            (ptr as usize) - (self.memory_base as usize)
        }
    }
    
    // Free (Default Allocator Only, Tiered uses reset)
    #[wasm_bindgen]
    pub fn free(&mut self, offset: usize) {
        if offset == 0 {
            return;
        }

        let ptr = unsafe { self.memory_base.add(offset) };

        let is_valid = match &self.strategy {
            AllocatorStrategy::Default(allocator) => {
                allocator.is_ptr_in_heap(ptr)
            },
            _ => false,
        };
        
        if !is_valid {
            return;
        }
        
        if let AllocatorStrategy::Default(allocator) = &mut self.strategy {
            allocator.free(ptr);
        }
    }
    
    // Copy data from JS to WASM memory
    #[wasm_bindgen]
    pub fn copy_from_js(&mut self, offset: usize, data: &js_sys::Uint8Array) -> Result<(), JsValue> {
        let data_len = data.length() as usize;
        if offset + data_len > self.memory_size {
            return Err(JsValue::from_str("Memory access out of bounds"));
        }
        
        unsafe {
            let dest_ptr = self.memory_base.add(offset);
            let dest_slice = std::slice::from_raw_parts_mut(dest_ptr, data_len);
            data.copy_to(dest_slice);
            Ok(())
        }
    }
    
    // Copy data from WASM memory to JS
    #[wasm_bindgen]
    pub fn copy_to_js(&self, offset: usize, length: usize) -> Result<js_sys::Uint8Array, JsValue> {
        self.get_memory_view(offset, length)
    }
    
    // Memory statistics
    #[wasm_bindgen]
    pub fn memory_stats(&self) -> js_sys::Object {
        let obj = js_sys::Object::new();
        
        // Get current memory size from WebAssembly directly
        let current_pages = core::arch::wasm32::memory_size(0);
        let current_size = current_pages * 65536;
        
        // Track total in-use memory
        let mut total_in_use = 0;
        
        // Add tier information if using tiered allocator
        if let AllocatorStrategy::Tiered(allocator) = &self.strategy {
            let tiers = js_sys::Array::new();
            
            for tier_num in 0..3 {
                if let Some(tier) = Tier::from_u8(tier_num) {
                    let (used, capacity, high_water, total_allocated) = allocator.tier_stats(tier);
                    let tier_obj = js_sys::Object::new();
                    
                    // Add current usage to total
                    total_in_use += used;
                    
                    js_sys::Reflect::set(
                        &tier_obj,
                        &JsValue::from_str("name"),
                        &JsValue::from_str(match tier {
                            Tier::Render => "render",
                            Tier::Scene => "scene",
                            Tier::Entity => "entity",
                        })
                    ).unwrap();
                    
                    js_sys::Reflect::set(
                        &tier_obj,
                        &JsValue::from_str("used"),
                        &JsValue::from_f64(used as f64)
                    ).unwrap();
                    
                    js_sys::Reflect::set(
                        &tier_obj,
                        &JsValue::from_str("capacity"),
                        &JsValue::from_f64(capacity as f64)
                    ).unwrap();
                    
                    js_sys::Reflect::set(
                        &tier_obj,
                        &JsValue::from_str("highWaterMark"),
                        &JsValue::from_f64(high_water as f64)
                    ).unwrap();
                    
                    js_sys::Reflect::set(
                        &tier_obj,
                        &JsValue::from_str("totalAllocated"),
                        &JsValue::from_f64(total_allocated as f64)
                    ).unwrap();
                    
                    // Calculate memory savings
                    let saved = if total_allocated > used {
                        total_allocated - used
                    } else {
                        0
                    };
                    
                    js_sys::Reflect::set(
                        &tier_obj,
                        &JsValue::from_str("memorySaved"),
                        &JsValue::from_f64(saved as f64)
                    ).unwrap();
                    
                    tiers.push(&tier_obj);
                }
            }
            
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str("tiers"),
                &tiers
            ).unwrap();
        } else if let AllocatorStrategy::Default(_) = &self.strategy {
            // For default allocator, we don't have tiered tracking
            // so we can't calculate total_in_use from tiers
            total_in_use = current_size; // Conservative estimate
        }
        
        // Set the total size to the in-use memory (not just raw WASM memory size)
        js_sys::Reflect::set(
            &obj, 
            &JsValue::from_str("totalSize"), 
            &JsValue::from_f64(total_in_use as f64)
        ).unwrap();
        
        // Add raw memory pages info
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("pages"),
            &JsValue::from_f64(current_pages as f64)
        ).unwrap();
        
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("rawMemorySize"),
            &JsValue::from_f64(current_size as f64)
        ).unwrap();
        
        // Add allocator type
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("allocatorType"),
            &JsValue::from_str(match &self.strategy {
                AllocatorStrategy::Default(_) => "default",
                AllocatorStrategy::Tiered(_) => "tiered",
            })
        ).unwrap();
        
        // Add useful utilization percentage
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("memoryUtilization"),
            &JsValue::from_f64((total_in_use as f64 / current_size as f64) * 100.0)
        ).unwrap();
        
        obj
    }
}