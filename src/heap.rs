/*
 * SPDX-FileCopyrightText: 2026 Jerin Joy
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::lock::Spinlock;

/// Represents a single chunk of memory in our linked-list allocator.
/// This translates the `struct memchunk` from C.
#[repr(C)]
pub struct MemChunk {
    pub next: *mut MemChunk,
    pub size: usize, // In C this was uint64_t, usize is native pointer size
}

/// Holds the state for a single heap region.
/// Note that we removed `lock: spinlock_t` from inside the struct!
/// In Rust, we wrap the entire `HeapInfo` in a `Spinlock<HeapInfo>` instead,
/// ensuring we can't accidentally access the fields without locking first.
pub struct HeapInfo {
    pub backing_memory: u8,
    pub memory_type: u8,
    pub head: *mut MemChunk,
    pub last_allocated: *mut MemChunk,
    pub size: usize,
    pub setup_done: bool,
}

// We also had constants in C, let's bring them over too:
pub const BACKING_MEMORY_DDR: u8 = 1;
pub const MEMORY_TYPE_WB: u8 = 3;
pub const MEMORY_TYPE_WC: u8 = 1;
pub const MEMORY_TYPE_UC: u8 = 0;

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub fn backing_memory_to_string(mem: u8) -> &'static str {
    match mem {
        BACKING_MEMORY_DDR => "DDR",
        _ => "UNKNOWN",
    }
}

#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub fn memory_type_to_string(mem: u8) -> &'static str {
    match mem {
        MEMORY_TYPE_WB => "WB",
        MEMORY_TYPE_WC => "WC",
        MEMORY_TYPE_UC => "UC",
        _ => "UNKNOWN",
    }
}

// The MSB of size indicates if a chunk is used
pub const MEMCHUNK_USED: usize = 0x8000_0000_0000_0000;
pub const MEMCHUNK_MAX_SIZE: usize = MEMCHUNK_USED - 1;
pub const MIN_HEAP_ALLOCATION_SIZE: usize = 8;
pub const PER_HEAP_ALLOCATION_METADATA_SIZE: usize = core::mem::size_of::<MemChunk>();
pub const MIN_HEAP_SEGMENT_BYTES: usize =
    PER_HEAP_ALLOCATION_METADATA_SIZE + MIN_HEAP_ALLOCATION_SIZE;

pub struct ChunkIterator {
    current: *mut MemChunk,
    start: *mut MemChunk,
    head: *mut MemChunk,
    second_pass: bool,
}

impl ChunkIterator {
    #[unsafe(link_section = ".jumpstart.cpu.text.smode")]
    pub fn new(head: *mut MemChunk, last_allocated: *mut MemChunk) -> Self {
        let mut start = if !last_allocated.is_null() {
            unsafe { (*last_allocated).next }
        } else {
            head
        };
        if start.is_null() {
            start = head;
        }
        Self {
            current: start,
            start,
            head,
            second_pass: false,
        }
    }

    #[unsafe(link_section = ".jumpstart.cpu.text.smode")]
    pub fn next_chunk(&mut self) -> *mut MemChunk {
        if self.current.is_null() {
            if !self.second_pass && self.start != self.head {
                self.second_pass = true;
                self.current = self.head;
            } else {
                return core::ptr::null_mut();
            }
        }

        if self.second_pass && self.current == self.start {
            return core::ptr::null_mut();
        }

        let result = self.current;
        self.current = unsafe { (*self.current).next };
        result
    }
}

pub const NUM_HEAPS_SUPPORTED: usize = 3;

pub struct JumpStartHeaps {
    heaps: [Spinlock<HeapInfo>; NUM_HEAPS_SUPPORTED],
}

impl JumpStartHeaps {
    #[unsafe(link_section = ".jumpstart.cpu.text.smode")]
    pub const fn new() -> Self {
        Self {
            heaps: [
                Spinlock::new(HeapInfo {
                    backing_memory: BACKING_MEMORY_DDR,
                    memory_type: MEMORY_TYPE_WB,
                    head: core::ptr::null_mut(),
                    last_allocated: core::ptr::null_mut(),
                    size: 0,
                    setup_done: false,
                }),
                Spinlock::new(HeapInfo {
                    backing_memory: BACKING_MEMORY_DDR,
                    memory_type: MEMORY_TYPE_WC,
                    head: core::ptr::null_mut(),
                    last_allocated: core::ptr::null_mut(),
                    size: 0,
                    setup_done: false,
                }),
                Spinlock::new(HeapInfo {
                    backing_memory: BACKING_MEMORY_DDR,
                    memory_type: MEMORY_TYPE_UC,
                    head: core::ptr::null_mut(),
                    last_allocated: core::ptr::null_mut(),
                    size: 0,
                    setup_done: false,
                }),
            ],
        }
    }

    #[unsafe(link_section = ".jumpstart.cpu.text.smode")]
    pub unsafe fn setup_heap(
        &self,
        heap_start: usize,
        heap_end: usize,
        backing_memory: u8,
        memory_type: u8,
    ) {
        for heap_lock in &self.heaps {
            let mut heap = heap_lock.lock();

            if heap.backing_memory != backing_memory || heap.memory_type != memory_type {
                continue;
            }

            if heap.setup_done {
                if heap.head != heap_start as *mut MemChunk {
                    panic!(
                        "Error: Heap already initialized for {}/{} with different range.",
                        backing_memory_to_string(backing_memory),
                        memory_type_to_string(memory_type)
                    );
                }
                return; // nothing to do, we're good
            }

            // TODO: Add the translation check!

            heap.head = heap_start as *mut MemChunk;
            heap.last_allocated = core::ptr::null_mut();
            unsafe {
                (*heap.head).next = core::ptr::null_mut();
                (*heap.head).size = heap_end - heap_start - PER_HEAP_ALLOCATION_METADATA_SIZE;
            }
            heap.size = heap_end - heap_start;

            heap.setup_done = true;
            return;
        }
    }
}

use core::alloc::{GlobalAlloc, Layout};

#[global_allocator]
#[unsafe(link_section = ".jumpstart.cpu.data.privileged")]
pub static HEAPS: JumpStartHeaps = JumpStartHeaps::new();

unsafe impl GlobalAlloc for JumpStartHeaps {
    #[unsafe(link_section = ".jumpstart.cpu.text.smode")]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { self.alloc_from_memory(layout, BACKING_MEMORY_DDR, MEMORY_TYPE_WB) }
    }

    #[unsafe(link_section = ".jumpstart.cpu.text.smode")]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { self.dealloc_from_memory(ptr, BACKING_MEMORY_DDR, MEMORY_TYPE_WB) }
    }
}

impl JumpStartHeaps {
    #[unsafe(link_section = ".jumpstart.cpu.text.smode")]
    pub unsafe fn alloc_from_memory(
        &self,
        layout: Layout,
        backing_memory: u8,
        memory_type: u8,
    ) -> *mut u8 {
        let size = layout.size();
        let alignment = layout.align();

        if size == 0 || size > MEMCHUNK_MAX_SIZE {
            return core::ptr::null_mut();
        }

        // Align alloc_size to MIN_HEAP_ALLOCATION_SIZE (which is 8)
        let alloc_size = (size + (MIN_HEAP_ALLOCATION_SIZE - 1)) & !(MIN_HEAP_ALLOCATION_SIZE - 1);

        for heap_lock in &self.heaps {
            let mut heap = heap_lock.lock();

            if heap.backing_memory != backing_memory || heap.memory_type != memory_type {
                continue;
            }

            if !heap.setup_done {
                return core::ptr::null_mut();
            }

            let mut iter = ChunkIterator::new(heap.head, heap.last_allocated);
            let pow2 = alignment.trailing_zeros();
            let mut aligned = false;
            let mut chunk = core::ptr::null_mut();
            let mut aligned_start = 0;
            let mut end = 0;

            loop {
                let c = iter.next_chunk();
                if c.is_null() {
                    break;
                }

                let c_size = unsafe { (*c).size };
                if (c_size & MEMCHUNK_USED) != 0 || c_size < alloc_size {
                    continue;
                }

                let start = (c as usize) + PER_HEAP_ALLOCATION_METADATA_SIZE;
                end = start + c_size;

                aligned_start = (((start.saturating_sub(1)) >> pow2) << pow2) + alignment;
                if start == aligned_start {
                    aligned = true;
                    chunk = c;
                    break;
                }

                let aligned_start_with_header =
                    ((((start + MIN_HEAP_SEGMENT_BYTES).saturating_sub(1)) >> pow2) << pow2)
                        + alignment;
                if aligned_start_with_header >= end {
                    continue;
                }
                if aligned_start_with_header + alloc_size > end {
                    continue;
                }

                aligned_start = aligned_start_with_header;
                chunk = c;
                break;
            }

            if chunk.is_null() {
                continue;
            }

            if !aligned {
                unsafe {
                    let new_chunk =
                        (aligned_start - PER_HEAP_ALLOCATION_METADATA_SIZE) as *mut MemChunk;
                    (*new_chunk).size = end - aligned_start;
                    (*new_chunk).next = (*chunk).next;
                    (*chunk).size -= (*new_chunk).size + PER_HEAP_ALLOCATION_METADATA_SIZE;
                    (*chunk).next = new_chunk;
                    chunk = (*chunk).next;
                }
            }

            unsafe {
                if (*chunk).size >= alloc_size + MIN_HEAP_SEGMENT_BYTES {
                    let new_chunk = ((chunk as usize)
                        + PER_HEAP_ALLOCATION_METADATA_SIZE
                        + alloc_size) as *mut MemChunk;
                    (*new_chunk).size =
                        (*chunk).size - alloc_size - PER_HEAP_ALLOCATION_METADATA_SIZE;
                    (*new_chunk).next = (*chunk).next;
                    (*chunk).next = new_chunk;
                    (*chunk).size = alloc_size;
                }

                (*chunk).size |= MEMCHUNK_USED;
            }
            heap.last_allocated = chunk;
            return ((chunk as usize) + PER_HEAP_ALLOCATION_METADATA_SIZE) as *mut u8;
        }

        core::ptr::null_mut()
    }

    #[unsafe(link_section = ".jumpstart.cpu.text.smode")]
    pub unsafe fn dealloc_from_memory(&self, ptr: *mut u8, backing_memory: u8, memory_type: u8) {
        if ptr.is_null() {
            return;
        }

        for heap_lock in &self.heaps {
            let mut heap = heap_lock.lock();

            if heap.backing_memory != backing_memory || heap.memory_type != memory_type {
                continue;
            }

            if !heap.setup_done {
                panic!("Error: Heap not initialized for dealloc");
            }

            let chunk = (ptr as usize - PER_HEAP_ALLOCATION_METADATA_SIZE) as *mut MemChunk;

            if chunk < heap.head || heap.head.is_null() {
                panic!("Error: Invalid free - address below heap start");
            }

            if heap.last_allocated == chunk {
                heap.last_allocated = core::ptr::null_mut();
            }

            unsafe {
                if ((*chunk).size & MEMCHUNK_USED) == 0 {
                    panic!("Error: Double free detected");
                }

                if ((*chunk).size & MEMCHUNK_MAX_SIZE) > MEMCHUNK_MAX_SIZE {
                    panic!("Error: Invalid chunk size in free");
                }

                // Mark as free
                (*chunk).size &= !MEMCHUNK_USED;
            }

            // Coalesce with next free chunk
            unsafe {
                if !(*chunk).next.is_null() && (((*(*chunk).next).size & MEMCHUNK_USED) == 0) {
                    (*chunk).size += (*(*chunk).next).size + PER_HEAP_ALLOCATION_METADATA_SIZE;
                    (*chunk).next = (*(*chunk).next).next;
                }
            }

            // Coalesce with previous free chunk
            let mut prev = core::ptr::null_mut();
            let mut curr = heap.head;
            unsafe {
                while !curr.is_null() && curr != chunk {
                    prev = curr;
                    curr = (*curr).next;
                }

                if !prev.is_null() && (((*prev).size & MEMCHUNK_USED) == 0) {
                    (*prev).size += (*chunk).size + PER_HEAP_ALLOCATION_METADATA_SIZE;
                    (*prev).next = (*chunk).next;
                }
            }
            return;
        }
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn setup_heap(
    heap_start: usize,
    heap_end: usize,
    backing_memory: u8,
    memory_type: u8,
) {
    unsafe { HEAPS.setup_heap(heap_start, heap_end, backing_memory, memory_type) };
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn malloc(size: usize) -> *mut u8 {
    if let Ok(layout) = Layout::from_size_align(size, MIN_HEAP_ALLOCATION_SIZE) {
        unsafe { HEAPS.alloc(layout) }
    } else {
        core::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    // Layout is ignored in our dealloc implementation
    let layout = Layout::from_size_align(0, 1).unwrap();
    unsafe { HEAPS.dealloc(ptr, layout) };
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn calloc(nmemb: usize, size: usize) -> *mut u8 {
    let total_size = nmemb.saturating_mul(size);
    unsafe {
        let ptr = malloc(total_size);
        if !ptr.is_null() {
            memset(ptr, 0, total_size);
        }
        ptr
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn memalign(alignment: usize, size: usize) -> *mut u8 {
    if !alignment.is_power_of_two() {
        return core::ptr::null_mut();
    }
    if let Ok(layout) = Layout::from_size_align(size, alignment) {
        unsafe { HEAPS.alloc(layout) }
    } else {
        core::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn malloc_from_memory(
    size: usize,
    backing_memory: u8,
    memory_type: u8,
) -> *mut u8 {
    if let Ok(layout) = Layout::from_size_align(size, MIN_HEAP_ALLOCATION_SIZE) {
        unsafe { HEAPS.alloc_from_memory(layout, backing_memory, memory_type) }
    } else {
        core::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn free_from_memory(ptr: *mut u8, backing_memory: u8, memory_type: u8) {
    unsafe { HEAPS.dealloc_from_memory(ptr, backing_memory, memory_type) };
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn calloc_from_memory(
    nmemb: usize,
    size: usize,
    backing_memory: u8,
    memory_type: u8,
) -> *mut u8 {
    let total_size = nmemb.saturating_mul(size);
    unsafe {
        let ptr = malloc_from_memory(total_size, backing_memory, memory_type);
        if !ptr.is_null() {
            memset(ptr, 0, total_size);
        }
        ptr
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn memalign_from_memory(
    alignment: usize,
    size: usize,
    backing_memory: u8,
    memory_type: u8,
) -> *mut u8 {
    if !alignment.is_power_of_two() {
        return core::ptr::null_mut();
    }
    if let Ok(layout) = Layout::from_size_align(size, alignment) {
        unsafe { HEAPS.alloc_from_memory(layout, backing_memory, memory_type) }
    } else {
        core::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    for i in 0..n {
        unsafe { *s.add(i) = c as u8 };
    }
    s
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".jumpstart.cpu.text.smode")]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    for i in 0..n {
        unsafe { *dest.add(i) = *src.add(i) };
    }
    dest
}

pub fn test_allocation() {
    // Bring dynamic allocation types into scope
    use alloc::boxed::Box;
    use alloc::string::String;
    use alloc::vec::Vec;

    crate::println!("--- Testing Dynamic Allocator ---");

    // Test Vec
    let mut my_vec = Vec::new();
    my_vec.push(10);
    my_vec.push(20);
    my_vec.push(30);
    crate::println!(
        "Vec successfully allocated! Sum = {}",
        my_vec.iter().sum::<i32>()
    );

    // Test Box
    let my_box = Box::new(42);
    crate::println!("Box successfully allocated! Value = {}", *my_box);

    // Test String
    let my_string = String::from("Hello from the Rust Heap!");
    crate::println!("String successfully allocated! String = {}", my_string);

    crate::println!("--- Dynamic Allocator Test Passed! ---");
}
