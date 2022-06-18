use core::alloc::Layout;

use crate::{
    utils::Locked,
    x86_64::{
        address::VirtualAddress,
        paging::{MappingError, OffsetMemoryMapper, Page, PageInner, PageTableEntryFlags},
    },
};

use self::bump::BumpAllocator;

pub mod bump;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[global_allocator]
static ALLOCATOR: Locked<BumpAllocator> = Locked::new(BumpAllocator::new());

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

    unsafe { ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE) };

    Ok(())
}
