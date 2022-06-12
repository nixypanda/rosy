use core::arch::asm;

use super::rflags::RFlags;

/// Enable interrupts.
///
/// This is a wrapper around the `sti` instruction.
pub fn enable() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

/// Disable interrupts.
///
/// This is a wrapper around the `cli` instruction.
fn disable() {
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

fn are_enabled() -> bool {
    RFlags::read().contains(RFlags::INTERRUPT_FLAG)
}

pub fn execute_without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let old_interrupt_state = are_enabled();

    if old_interrupt_state {
        disable();
    }

    let ret = f();

    if old_interrupt_state {
        enable();
    }

    ret
}

pub fn invoke_breakpoint_exception() {
    // Cause a breakpoint exception by invoking the `int3` instruction.
    // https://en.wikipedia.org/wiki/INT_%28x86_instruction%29
    unsafe { asm!("int3", options(nomem, nostack)) }
}
