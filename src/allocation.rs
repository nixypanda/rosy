//! Setup heap allocation
//!
//! We setup a [`global_allocator`] here. Which uses an implementaion of Allocator (currently
//! ['LinkedListAllocator']). Also, provides functionality to initialize the heap space.

use core::alloc::Layout;

use crate::{
    allocator::linked_list::LinkedListAllocator,
    utils::Locked,
    x86_64::{
        address::VirtualAddress,
        paging::{MappingError, OffsetMemoryMapper, Page, PageInner, PageTableEntryFlags},
    },
};

/// Easily recognizable heap starting address.
pub const HEAP_START: usize = 0x_4444_4444_0000;
/// Heap Size
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

/// Setup a global heap allocator. This attribute is only appliable to a `static` that implements
/// the [`GlobalAlloc`] trait.
#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

/// Setup which function should be called when our global allocator fails to allocate space on the
/// heap.
#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

/// Setup virtual memory range and map it to physical memory.
pub fn init_heap(mapper: &mut OffsetMemoryMapper) -> Result<(), MappingError> {
    let page_range = {
        let heap_start = VirtualAddress::new(HEAP_START as u64);
        let heap_end = heap_start + (HEAP_SIZE as u64 - 1u64);
        let heap_start_page = PageInner::containing_address(heap_start);
        let heap_end_page = PageInner::containing_address(heap_end);
        PageInner::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let page = Page::Normal(page);
        let frame = mapper
            .frame_allocator
            .allocate_normal_frame()
            .ok_or(MappingError::FrameAllocationFailed)?;
        let flags = PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags)? };
    }

    unsafe { ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE) };

    Ok(())
}
