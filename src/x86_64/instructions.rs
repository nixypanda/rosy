use core::arch::asm;

use super::addr::VirtualAddress;

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
