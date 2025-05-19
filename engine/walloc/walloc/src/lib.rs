use wasm_bindgen::prelude::*;
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};

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

// Memory block header structure
#[repr(C)]
struct BlockHeader {
    size: usize,
    next: *mut BlockHeader,
    is_free: bool,
    tier: u8,
}

// Available tiers in our hierarchy
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

// Base allocator implementation (same as your DefaultAllocator)
pub struct DefaultAllocator {
    free_list_head: *mut BlockHeader,
    heap_start: *mut u8,
    heap_end: *mut u8,
}

// A single arena in our tiered system
pub struct Arena {
    base: *mut u8,
    size: usize,
    current_offset: AtomicUsize,
    tier: Tier,
}

// Entity that owns memory in an arena
pub struct MemoryOwner {
    // The arena this entity belongs to
    arena: Arc<Mutex<Arena>>,
    // Memory regions this entity owns (offset, size)
    allocations: Vec<(usize, usize)>,
}

// TieredAllocator manages multiple arenas
pub struct TieredAllocator {
    // The fallback allocator for when arenas are full
    fallback: DefaultAllocator,
    
    // Our three tiers of arenas
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
        }
    }
    
    // Bump allocation - very fast but no individual deallocation
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
                    // Success! Return pointer to the allocated memory
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
}

// TieredAllocator implementation
impl TieredAllocator {
    pub fn new(memory_base: *mut u8, memory_size: usize) -> Self {
        // Calculate sizes for each arena
        // Render tier: 50% of memory, Scene tier: 30%, Entity tier: 15%, Default: 5%
        let render_size = (memory_size * 50) / 100;
        let scene_size = (memory_size * 30) / 100;
        let entity_size = (memory_size * 15) / 100;
        let default_size = memory_size - render_size - scene_size - entity_size;
        
        // Create arenas
        let render_base = memory_base;
        let scene_base = unsafe { render_base.add(render_size) };
        let entity_base = unsafe { scene_base.add(scene_size) };
        let default_base = unsafe { entity_base.add(entity_size) };
        
        let render_arena = Arena::new(render_base, render_size, Tier::Render);
        let scene_arena = Arena::new(scene_base, scene_size, Tier::Scene);
        let entity_arena = Arena::new(entity_base, entity_size, Tier::Entity);
        
        // Create fallback allocator
        let fallback = DefaultAllocator::new(default_base, default_size);
        
        TieredAllocator {
            fallback,
            render_arena: Arc::new(Mutex::new(render_arena)),
            scene_arena: Arc::new(Mutex::new(scene_arena)),
            entity_arena: Arc::new(Mutex::new(entity_arena)),
        }
    }
    
    // Allocate memory from the specified tier and return a memory owner
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
        
        // If the arena allocation failed, return None
        None
    }
    
    // Allocate memory from the specified tier (fallback to default allocator)
    pub fn allocate(&mut self, size: usize, tier: Tier) -> *mut u8 {
        // First try to allocate with the tiered approach
        if let Some((_, ptr)) = self.allocate_with_owner(size, tier) {
            return ptr;
        }
        
        // If that fails, fallback to the default allocator
        self.fallback.malloc(size)
    }
    
    // Free memory - this is a no-op for arena allocations
    // Only works for fallback allocations
    pub fn free(&mut self, ptr: *mut u8) {
        // Check which arena this pointer belongs to
        if self.is_ptr_in_arena(ptr) {
            // Do nothing for arena allocations - they're freed as a group
            // or when the MemoryOwner is dropped
            return;
        }
        
        // Use fallback free for non-arena allocations
        self.fallback.free(ptr);
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
    
    // Get stats for a tier
    pub fn tier_stats(&self, tier: Tier) -> (usize, usize) {
        match tier {
            Tier::Render => {
                if let Ok(arena) = self.render_arena.lock() {
                    (arena.usage(), arena.capacity())
                } else {
                    (0, 0)
                }
            },
            Tier::Scene => {
                if let Ok(arena) = self.scene_arena.lock() {
                    (arena.usage(), arena.capacity())
                } else {
                    (0, 0)
                }
            },
            Tier::Entity => {
                if let Ok(arena) = self.entity_arena.lock() {
                    (arena.usage(), arena.capacity())
                } else {
                    (0, 0)
                }
            },
        }
    }
    
    // Check if a pointer is valid
    pub fn is_ptr_valid(&self, ptr: *mut u8) -> bool {
        self.is_ptr_in_arena(ptr) || self.fallback.is_ptr_in_heap(ptr)
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
        let memory_base = unsafe { 
            core::arch::wasm32::memory_size(0) as *mut u8
        };
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
            AllocatorStrategy::Default(allocator) => {
                // Fallback to regular allocation
                allocator.malloc(size)
            },
        };

        self.memory_size = core::arch::wasm32::memory_size(0) * 65536;
        
        // Return offset from memory base
        if ptr.is_null() {
            0 // Error case, return 0 (null) pointer
        } else {
            (ptr as usize) - (self.memory_base as usize)
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
    
    // Get statistics for a specific tier
    #[wasm_bindgen]
    pub fn tier_stats(&self, tier_number: u8) -> js_sys::Object {
        let obj = js_sys::Object::new();
        
        if let Some(tier) = Tier::from_u8(tier_number) {
            if let AllocatorStrategy::Tiered(allocator) = &self.strategy {
                let (used, capacity) = allocator.tier_stats(tier);
                
                js_sys::Reflect::set(
                    &obj, 
                    &JsValue::from_str("used"), 
                    &JsValue::from_f64(used as f64)
                ).unwrap();
                
                js_sys::Reflect::set(
                    &obj,
                    &JsValue::from_str("capacity"),
                    &JsValue::from_f64(capacity as f64)
                ).unwrap();
                
                js_sys::Reflect::set(
                    &obj,
                    &JsValue::from_str("tierName"),
                    &JsValue::from_str(match tier {
                        Tier::Render => "render",
                        Tier::Scene => "scene",
                        Tier::Entity => "entity",
                    })
                ).unwrap();
            }
        }
        
        obj
    }
    
    // Standard allocate (backward compatibility)
    #[wasm_bindgen]
    pub fn allocate(&mut self, size: usize) -> usize {
        // For backward compatibility, use regular malloc in default mode
        let ptr = match &mut self.strategy {
            AllocatorStrategy::Default(allocator) => {
                allocator.malloc(size)
            },
            AllocatorStrategy::Tiered(allocator) => {
                allocator.allocate(size, Tier::Entity)
            },
        };

        self.memory_size = core::arch::wasm32::memory_size(0) * 65536;
        
        // Return offset from memory base
        if ptr.is_null() {
            0 // Error case, return 0 (null) pointer
        } else {
            (ptr as usize) - (self.memory_base as usize)
        }
    }
    
    // Free previously allocated memory
    #[wasm_bindgen]
    pub fn free(&mut self, offset: usize) {
        if offset == 0 {
            return; // Null pointer, nothing to free
        }

        let ptr = unsafe { self.memory_base.add(offset) };

        // Check if pointer is valid
        let is_valid = match &self.strategy {
            AllocatorStrategy::Default(allocator) => {
                allocator.is_ptr_in_heap(ptr)
            },
            AllocatorStrategy::Tiered(allocator) => {
                allocator.is_ptr_valid(ptr)
            },
        };
        
        if !is_valid {
            return; // out of bounds
        }
        
        match &mut self.strategy {
            AllocatorStrategy::Default(allocator) => {
                allocator.free(ptr);
            },
            AllocatorStrategy::Tiered(allocator) => {
                allocator.free(ptr); // No-op for arena allocations
            },
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
        
        js_sys::Reflect::set(
            &obj, 
            &JsValue::from_str("totalSize"), 
            &JsValue::from_f64(current_size as f64)
        ).unwrap();
        
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("pages"),
            &JsValue::from_f64(current_pages as f64)
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
        
        // Add tier information if using tiered allocator
        if let AllocatorStrategy::Tiered(allocator) = &self.strategy {
            let tiers = js_sys::Array::new();
            
            for tier_num in 0..3 {
                if let Some(tier) = Tier::from_u8(tier_num) {
                    let (used, capacity) = allocator.tier_stats(tier);
                    let tier_obj = js_sys::Object::new();
                    
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
                        &JsValue::from_str("percentage"),
                        &JsValue::from_f64((used as f64 / capacity as f64) * 100.0)
                    ).unwrap();
                    
                    tiers.push(&tier_obj);
                }
            }
            
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str("tiers"),
                &tiers
            ).unwrap();
        }
        
        obj
    }
}