use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};

use crate::x86_64::{
    address::VirtualAddress,
    paging::{MappingError, OffsetMemoryMapper, Page, PageInner, PageTableEntryFlags},
};

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

pub struct NullAllocator;

unsafe impl GlobalAlloc for NullAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should never be called for NullAllocator");
    }
}

#[global_allocator]
pub static NO_USE_ALLOCATOR: NullAllocator = NullAllocator;

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

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

    Ok(())
}
