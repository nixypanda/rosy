//! Memory related operations

use crate::x86_64::{
    address::{PhysicalAddress, VirtualAddress},
    instructions::read_control_register_3,
    paging::{MappedFrame, PageTable, PageTableFrame, PageTableLevel, Size4KiB},
};

pub struct OffsetMemoryMapper {
    physical_memory_offset: VirtualAddress,
    l4_table_address: PageTableFrame<Size4KiB>,
}

impl OffsetMemoryMapper {
    pub fn new(physical_memory_offset: VirtualAddress) -> Self {
        let (level4_table_physical_address, _) = read_control_register_3();
        OffsetMemoryMapper {
            physical_memory_offset,
            l4_table_address: level4_table_physical_address,
        }
    }

    unsafe fn frame_to_pointer(&self, frame: MappedFrame) -> *mut PageTable {
        let virtual_address = self.physical_memory_offset + frame.start_address().as_u64();
        virtual_address.as_mut_ptr()
    }

    pub fn translate_address(&self, address: VirtualAddress) -> Option<PhysicalAddress> {
        let cr3_frame = MappedFrame::Normal(self.l4_table_address);

        let l4_table = unsafe { &*(self.frame_to_pointer(cr3_frame)) };
        let entry = &l4_table[address.p4_index()];
        let l4_frame: MappedFrame = entry.frame(PageTableLevel::Level4).ok()?;

        let l3_table = unsafe { &*(self.frame_to_pointer(l4_frame)) };
        let entry = &l3_table[address.p3_index()];
        let l3_frame: MappedFrame = entry.frame(PageTableLevel::Level3).ok()?;
        if l3_frame.is_huge() {
            return Some(l3_frame.address_at_offset(address.page_offset(PageTableLevel::Level3)));
        }

        let l2_table = unsafe { &*(self.frame_to_pointer(l3_frame)) };
        let entry = &l2_table[address.p2_index()];
        let l2_frame: MappedFrame = entry.frame(PageTableLevel::Level2).ok()?;
        if l2_frame.is_huge() {
            return Some(l2_frame.address_at_offset(address.page_offset(PageTableLevel::Level2)));
        }

        let l1_table = unsafe { &*(self.frame_to_pointer(l2_frame)) };
        let entry = &l1_table[address.p1_index()];
        let l1_frame: MappedFrame = entry.frame(PageTableLevel::Level1).ok()?;

        Some(l1_frame.address_at_offset(address.page_offset(PageTableLevel::Level1)))
    }
}

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
