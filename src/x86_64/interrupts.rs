//! Utility operations for handling interrupts in general

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
pub fn disable() {
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

/// Atomically enable interrupts and put the CPU to sleep
///
/// Executes the `sti; hlt` instruction sequence. Since the `sti` instruction
/// keeps interrupts disabled until after the immediately following
/// instruction (called "interrupt shadow"), no interrupt can occur between the
/// two instructions. (One exception to this are non-maskable interrupts; this
/// is explained below.)
#[inline]
pub fn enable_and_halt_cpu_till_next_one() {
    unsafe {
        asm!("sti; hlt", options(nomem, nostack));
    }
}

fn are_enabled() -> bool {
    RFlags::read().contains(RFlags::INTERRUPT_FLAG)
}

/// Run a closure with disabled interrupts.
///
/// Run the given closure, disabling interrupts before running it (if they aren't already disabled).
/// Afterwards, interrupts are enabling again if they were enabled before.
///
/// Nesting can result in undefined behavior.
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

/// Cause a breakpoint exception by invoking the `int3` instruction.
pub fn invoke_breakpoint_exception() {
    // Cause a breakpoint exception by invoking the `int3` instruction.
    // https://en.wikipedia.org/wiki/INT_%28x86_instruction%29
    unsafe { asm!("int3", options(nomem, nostack)) }
}
