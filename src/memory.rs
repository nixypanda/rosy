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
        level4_table_physical_address + physical_memory_offset.as_u64();

    let page_table_pointer: *mut PageTable = level4_table_virtual_address.as_mut_ptr();
    &mut *page_table_pointer
}
