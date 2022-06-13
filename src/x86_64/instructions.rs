use core::arch::asm;

pub fn halt_cpu_till_next_interrupt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}
