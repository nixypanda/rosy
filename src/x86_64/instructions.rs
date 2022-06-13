use bitflags::bitflags;
use core::arch::asm;

use super::addr::{PhysicalAddress, VirtualAddress};

pub fn halt_cpu_till_next_interrupt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

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

pub fn read_control_register_3() -> (PhysicalAddress, Cr3Flags) {
    let mut cr3: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }
    let physical_adderss = PhysicalAddress::new(cr3 & 0x_000f_ffff_ffff_f000);
    let flags = Cr3Flags::from_bits_truncate(cr3 & 0xfff);

    (physical_adderss, flags)
}
