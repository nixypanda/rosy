//! Memory related operations

use crate::x86_64::{
    address::{PhysicalAddress, VirtualAddress},
    instructions::read_control_register_3,
    paging::{FrameError, PageTable},
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

pub fn translate_address(
    address: VirtualAddress,
    physical_memory_offset: VirtualAddress,
) -> Option<PhysicalAddress> {
    let (level_4_table_frame, _) = read_control_register_3();

    let table_indices = [
        address.p4_index(),
        address.p3_index(),
        address.p2_index(),
        address.p1_index(),
    ];

    let mut frame = level_4_table_frame;

    for &index in &table_indices {
        let virtual_address = physical_memory_offset + frame.start_address().as_u64();
        let table = unsafe { &mut *(virtual_address.as_mut_ptr() as *mut PageTable) };

        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    Some(frame.start_address() + u64::from(address.page_offset()))
}
