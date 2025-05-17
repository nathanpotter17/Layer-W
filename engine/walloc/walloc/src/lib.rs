use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Walloc {
    strategy: OptionAllocatorStrategy,
    memory_base: *mut u8, // our base unit is 8, which keeps our 32 bit address space comfortable. WASM / JS also favors UInt8 and Float32 Arrays.
    memory_size: usize, // the total size of our memory.
}

pub enum OptionAllocatorStrategy {
    Default(DefaultAllocator),
}

// Memory block header structure
#[repr(C)] // replicate c struct. no reordering.
struct BlockHeader {
    size: usize,       // Size of the block (including header)
    next: *mut BlockHeader, // Pointer to the next free block (if in free list)
    is_free: bool,     // Whether this block is free
}

pub struct DefaultAllocator {
    free_list_head: *mut BlockHeader,
    heap_start: *mut u8,
    heap_end: *mut u8,
}

pub struct Wallocator {
    free_list_head: *mut BlockHeader,
    heap_start: *mut u8,
    heap_end: *mut u8,
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

#[wasm_bindgen]
impl Walloc {
    pub fn new() -> Self {
        let memory_base = core::arch::wasm32::memory_size(0) as *mut u8;
        let memory_size = (core::arch::wasm32::memory_size(0) * 65536) as usize;
        
        // Allocate the total available WASM memory, up to the max. This will always request the max amount of memory, so be cautious.
        let allocator = DefaultAllocator::new(memory_base, memory_size);
        
        Walloc {
            strategy: OptionAllocatorStrategy::Default(allocator),
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
    
    // Read a 32-bit integer from memory
    #[wasm_bindgen]
    pub fn read_u32(&self, offset: usize) -> Result<u32, JsValue> {
        if offset + 4 > self.memory_size {
            return Err(JsValue::from_str("Memory access out of bounds"));
        }
        
        unsafe {
            let ptr = self.memory_base.add(offset) as *const u32;
            Ok(*ptr)
        }
    }
    
    // Write a 32-bit integer to memory
    #[wasm_bindgen]
    pub fn write_u32(&mut self, offset: usize, value: u32) -> Result<(), JsValue> {
        if offset + 4 > self.memory_size {
            return Err(JsValue::from_str("Memory access out of bounds"));
        }
        
        unsafe {
            let ptr = self.memory_base.add(offset) as *mut u32;
            *ptr = value;
            Ok(())
        }
    }
    
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
        
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("allocatorType"),
            &JsValue::from_str("default")
        ).unwrap();
        
        obj
    }
    
    // Allocate a chunk of memory of given size
    #[wasm_bindgen]
    pub fn allocate(&mut self, size: usize) -> usize {
        let ptr = match &mut self.strategy {
            OptionAllocatorStrategy::Default(allocator) => {
                allocator.malloc(size)
            },
        };

        self.memory_size = core::arch::wasm32::memory_size(0) * 65536;
        
        // Return offset from memory base instead of raw pointer for JS safety (i.e; JS can use this value as an index into its typed arrays (or no? idk))
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


        // Check if pointer is in heap bounds by accessing through the strategy
        let in_heap = match &mut self.strategy {
            OptionAllocatorStrategy::Default(allocator) => {
                allocator.is_ptr_in_heap(ptr)
            },
        };
        
        if !in_heap {
            return; // out of bounds
        }
        
        
        match &mut self.strategy {
            OptionAllocatorStrategy::Default(allocator) => {
                allocator.free(ptr);
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
    
    // Realloc - resize an existing allocation
    #[wasm_bindgen]
    pub fn realloc(&mut self, offset: usize, old_size: usize, new_size: usize) -> usize {
        if offset == 0 {
            // If the pointer is null, this is just a malloc
            return self.allocate(new_size);
        }
        
        if new_size == 0 {
            // If new size is 0, this is just a free
            self.free(offset);
            return 0;
        }
        
        // Allocate new block
        let new_offset = self.allocate(new_size);
        if new_offset == 0 {
            // Allocation failed
            return 0;
        }
        
        // Copy data from old location to new
        let copy_size = if old_size < new_size { old_size } else { new_size };
        
        unsafe {
            let src_ptr = self.memory_base.add(offset);
            let dest_ptr = self.memory_base.add(new_offset);
            std::ptr::copy_nonoverlapping(src_ptr, dest_ptr, copy_size);
        }
        
        // Free the old allocation
        self.free(offset);
        
        new_offset
    }
}