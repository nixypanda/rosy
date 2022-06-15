//! Memory related operations

use crate::x86_64::{
    address::{PhysicalAddress, VirtualAddress},
    instructions::read_control_register_3,
    paging::{FrameError, MappedFrame, PageTable},
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
    let (l4_table_address, _) = read_control_register_3();

    let l4_virtual_address = physical_memory_offset + l4_table_address.start_address().as_u64();
    let l4_table = unsafe { &mut *(l4_virtual_address.as_mut_ptr() as *mut PageTable) };
    let entry = &l4_table[address.p4_index()];
    if entry.is_huge() {
        panic!("Can't be huge page at Level 4");
    }
    let l4_frame: MappedFrame = entry.frame().ok()?;

    let l3_virtual_address = physical_memory_offset + l4_frame.start_address().as_u64();
    let l3_table = unsafe { &mut *(l3_virtual_address.as_mut_ptr() as *mut PageTable) };
    let entry = &l3_table[address.p3_index()];
    if entry.is_huge() {
        panic!("1GiB huge pages are unspported");
    }
    let l3_frame: MappedFrame = entry.frame().ok()?;

    let l2_virtual_address = physical_memory_offset + l3_frame.start_address().as_u64();
    let l2_table = unsafe { &mut *(l2_virtual_address.as_mut_ptr() as *mut PageTable) };
    let entry = &l2_table[address.p2_index()];
    if entry.is_huge() {
        let l2_frame: MappedFrame = entry.frame().ok()?;
        return Some(l2_frame.start_address() + (address.as_u64() & 0o000_000_000_777_7777));
    }
    let l2_frame: MappedFrame = entry.frame().ok()?;

    let l1_virtual_address = physical_memory_offset + l2_frame.start_address().as_u64();
    let l1_table = unsafe { &mut *(l1_virtual_address.as_mut_ptr() as *mut PageTable) };
    let entry = &l1_table[address.p1_index()];
    if entry.is_huge() {
        panic!("Can't have huge page at Level 1");
    }
    let l1_frame: MappedFrame = entry.frame().ok()?;

    Some(l1_frame.start_address() + u64::from(address.page_offset()))
}
