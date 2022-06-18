use crate::utils::Locked;
use alloc::alloc::{GlobalAlloc, Layout};

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// We need to seprate this aout as the allocator implementation is required to be a static and
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

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump_allocator = self.lock();
        let alloc_start = bump_allocator.next;
        bump_allocator.next = alloc_start + layout.size();
        bump_allocator.allocations += 1;
        alloc_start as *mut u8
    }
    unsafe fn dealloc(&self, layout: *mut u8, _: Layout) {
        let mut bump_allocator = self.lock();
        bump_allocator.allocations -= 1;

        if bump_allocator.allocations == 0 {
            bump_allocator.next = bump_allocator.heap_start;
        }
    }
}
