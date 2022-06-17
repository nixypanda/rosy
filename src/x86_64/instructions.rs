//! Special x86_64 instructions.

use bitflags::bitflags;
use core::arch::asm;

use super::{
    address::{PhysicalAddress, VirtualAddress},
    paging::PageFrame,
};

const CR3_PHYSICAL_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
const CR3_FLAGS_MASK: u64 = 0xfff;

/// Puts the CPU to sleep till it encounters the next interrupt. Calling in a loop can be
/// significantly less resourse intensive than a busy-loop.
pub fn halt_cpu_till_next_interrupt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Read the current age fault linear address from the CR2 register.
pub fn read_control_register_2() -> VirtualAddress {
    let mut cr2: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));
    }
    VirtualAddress::new(cr2)
}

bitflags! {
    /// Controls cache settings for the level 4 page table.
    pub struct Cr3Flags: u64 {
        /// Use a writethrough cache policy for the P4 table (else a writeback policy is used).
        const PAGE_LEVEL_WRITETHROUGH = 1 << 3;
        /// Disable caching for the P4 table.
        const PAGE_LEVEL_CACHE_DISABLE = 1 << 4;
    }
}

/// Read the current P4 table address from the CR3 register.
pub fn read_control_register_3() -> (PageFrame, Cr3Flags) {
    let mut cr3: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }
    u64_to_page_table_frame_and_cr3_flags(cr3)
}

pub unsafe fn write_control_register_3(page_frame: PageFrame, flags: Cr3Flags) {
    let addr = page_frame.start_address();
    let value = addr.as_u64() | (flags.bits() as u16) as u64;

    asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
}

fn u64_to_page_table_frame_and_cr3_flags(value: u64) -> (PageFrame, Cr3Flags) {
    let physical_address = PhysicalAddress::new(value & CR3_PHYSICAL_ADDRESS_MASK);
    let flags = Cr3Flags::from_bits_truncate(value & CR3_FLAGS_MASK);
    let page_table_frame = PageFrame::top_level_containing_address(physical_address);

    (page_table_frame, flags)
}

#[test_case]
fn test_u64_to_page_table_frame_and_cr3_flags() {
    let address: u64 = 0x0123_4567_89abc_def0;

    let (page_table_frame, flags) = u64_to_page_table_frame_and_cr3_flags(address);
    assert_eq!(
        page_table_frame,
        PageFrame::normal_from_raw(PhysicalAddress::from_raw(0x0000_4567_89abc_d000))
    );
    assert_eq!(flags, Cr3Flags::PAGE_LEVEL_CACHE_DISABLE);
}
