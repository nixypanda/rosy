use core::arch::asm;

use super::privilege_level::PrivilegeLevel;

/// Specifies which element to load into a segment from descriptor tables (i.e., is a index to LDT
/// or GDT table with some additional flags).
///
/// See Intel 3a, Section 3.4.2 "Segment Selectors"
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SegmentSelector(pub u16);

impl SegmentSelector {
    pub fn new(index: u16, requested_privilege_level: PrivilegeLevel) -> Self {
        SegmentSelector((index << 3) | (requested_privilege_level as u16))
    }
}

pub fn get_current_code_segment() -> SegmentSelector {
    let segment: u16;
    unsafe {
        asm!("mov {0:x}, cs", out(reg) segment, options(nomem, nostack, preserves_flags));
    }
    SegmentSelector(segment)
}

/// Note this is special since we cannot directly move to [`CS`]; x86 requires the instruction
/// pointer and [`CS`] to be set at the same time. To do this, we push the new segment selector
/// and return value onto the stack and use a "far return" (`retfq`) to reload [`CS`] and
/// continue at the end of our function.
///
/// Note we cannot use a "far call" (`lcall`) or "far jmp" (`ljmp`) to do this because then we
/// would only be able to jump to 32-bit instruction pointers. Only Intel implements support
/// for 64-bit far calls/jumps in long-mode, AMD does not.
pub unsafe fn set_code_segment_selector(sel: SegmentSelector) {
    asm!(
        "push {sel}",
        "lea {tmp}, [1f + rip]",
        "push {tmp}",
        "retfq",
        "1:",
        sel = in(reg) u64::from(sel.0),
        tmp = lateout(reg) _,
        options(preserves_flags),
    );
}
