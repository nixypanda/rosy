//! Simplest Allocator design
use core::ptr;

use crate::utils::Locked;
use alloc::alloc::{GlobalAlloc, Layout};

use super::align_up;

/// It allocates memory linearly and only keeps track of the number of allocated bytes and the
/// number of allocations.
///
/// # Idea
/// The idea behind a bump allocator is to linearly allocate memory by increasing (“bumping”) a
/// next variable, which points at the beginning of the unused memory. At the beginning, next is
/// equal to the start address of the heap. On each allocation, next is increased by the allocation
/// so that it always points to the boundary between used and unused memory.
/// ```text
///                     next
///                       │                                                                        
///  ┌────────────────────├───────────────────────────────────────────────────────────────┐
///  │ Allocated Region 1 │            free space                                         │
///  ├────────────────────└───────────────────────────────────────────────────────────────┘  
///  │                                                                                           
///  Heap Start                                                                             
///
///                                          next
///                                            │                                                  
///  ┌────────────────────┌────────────────────┤──────────────────────────────────────────┐
///  │ Allocated Region 1 │ Allocated Region 2 │            free space                    │
///  ├────────────────────└────────────────────┘──────────────────────────────────────────┘  
///  │                                                                                           
///  Heap Start                                                                             
///                                                                                              
///                                                                                              
/// ```
/// The next pointer only moves in a single direction and thus never hands out the same memory
/// region twice. When it reaches the end of the heap, no more memory can be allocated, resulting
/// in an out-of-memory error on the next allocation.
///
/// # Limitation(s)
/// The main limitation of a bump allocator is that it can only reuse deallocated memory after all
/// allocations have been freed. This means that a single long-lived allocation suffices to prevent
/// memory reuse.
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Create a new [`BumpAllocator`]
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// We need to seprate this out as the allocator implementation is required to be a static and
    /// we can't call a non-const function in a static initializer. So it the static we use the
    /// `new` method and then we call this method to initialize the allocator.
    ///
    /// # Safety
    /// This method is unsafe as the caller must ensure that the memory address provided for the
    /// heap address is vaild and will not lead to any memory related issuse.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

/// Implemenation of [`GlobalAlloc`] for a locked version of [`BumpAllocator`]. We need to do this
/// as the allocator we register using the `#[global_allocator]` macro is a static. `static`s in
/// rust are immutable so methods in this trait use reference to `self` and not `mut self`. Given
/// that we need to perform mutations we make use of syncornised interior mutability.
unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump_allocator = self.lock();

        let allocation_start = align_up(bump_allocator.next, layout.align());
        let allocation_end = match allocation_start.checked_add(layout.size()) {
            Some(next) => next,
            // Returning null pointer here signals that we ran out of memory. Here it can happen
            // when adding layout.size() to alloc_start results in overflow.
            None => return ptr::null_mut(),
        };
        if bump_allocator.next > bump_allocator.heap_end {
            // Signaling that we ran out of memory
            ptr::null_mut()
        } else {
            bump_allocator.allocations += 1;
            bump_allocator.next = allocation_end;
            allocation_start as *mut u8
        }
    }
    unsafe fn dealloc(&self, _layout: *mut u8, _: Layout) {
        let mut bump_allocator = self.lock();
        bump_allocator.allocations -= 1;

        if bump_allocator.allocations == 0 {
            bump_allocator.next = bump_allocator.heap_start;
        }
    }
}
