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
