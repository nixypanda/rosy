//! Memory related operations

use bootloader::BootInfo;

use crate::{
    allocation,
    x86_64::{
        address::VirtualAddress,
        instructions::read_control_register_3,
        paging::{FrameAllocator, OffsetMemoryMapper, PageTable},
    },
};

/// Get the level 4 page table
///
/// # Safety
/// The user needs to provide a vaild physical memory offset value for this to work properly
/// otherwise undefined memory behaviour can occur.
pub unsafe fn active_level4_page_table(
    physical_memory_offset: VirtualAddress,
) -> &'static mut PageTable {
    let (level4_table_physical_address, _) = read_control_register_3();
    let level4_table_virtual_address =
        level4_table_physical_address.start_address() + physical_memory_offset.as_u64();

    let page_table_pointer: *mut PageTable = level4_table_virtual_address.as_mut_ptr();
    &mut *page_table_pointer
}

/// Initialize memory system
///
/// * Sets up offset based memory mapping
/// * Sets up heap allocator.
pub fn init(boot_info: &'static BootInfo) {
    let physical_memory_offset = VirtualAddress::new(boot_info.physical_memory_offset);
    let offset_memory_mapper: &mut OffsetMemoryMapper = unsafe {
        &mut OffsetMemoryMapper::new(
            physical_memory_offset,
            FrameAllocator::new(&boot_info.memory_map),
        )
    };
    allocation::init_heap(offset_memory_mapper).expect("heap initialization failed");
}
