use core::arch::asm;

/// Enable interrupts.
///
/// This is a wrapper around the `sti` instruction.
#[inline]
pub fn enable() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}
