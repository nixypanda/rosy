use super::{addr::VirtualAddress, tss::TaskStateSegment};
use bit_field::BitField;
use bitflags::bitflags;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DescriptorTablePointer {
    /// Size of the DT.
    limit: u16,
    /// Pointer to the memory region containing the DT.
    base: VirtualAddress,
}

impl DescriptorTablePointer {
    pub fn new(base: VirtualAddress, limit: u16) -> DescriptorTablePointer {
        DescriptorTablePointer { base, limit }
    }
}

/// A 64-bit mode segment descriptor.
///
/// Segmentation is no longer supported in 64-bit mode, so most of the descriptor
/// contents are ignored.
#[derive(Debug, Clone)]
pub(crate) enum Descriptor {
    /// Descriptor for a code or data segment.
    ///
    /// Since segmentation is no longer supported in 64-bit mode, almost all of
    /// code and data descriptors is ignored. Only some flags are still used.
    UserSegment(u64),
    /// A system segment descriptor such as a LDT or TSS descriptor.
    SystemSegment(u64, u64),
}

impl Descriptor {
    /// Creates a segment descriptor for a 64-bit kernel code segment. Suitable
    /// for use with `syscall` or 64-bit `sysenter`.
    pub fn kernel_code_segment() -> Descriptor {
        Descriptor::UserSegment(DescriptorFlags::KERNEL_CODE64.bits())
    }

    pub(crate) fn tss_segment(tss: &TaskStateSegment) -> Descriptor {
        use core::mem::size_of;

        let ptr = tss as *const _ as u64;

        let mut low = DescriptorFlags::PRESENT.bits();
        // base
        low.set_bits(16..40, ptr.get_bits(0..24));
        low.set_bits(56..64, ptr.get_bits(24..32));
        // limit (the `-1` in needed since the bound is inclusive)
        low.set_bits(0..16, (size_of::<TaskStateSegment>() - 1) as u64);
        // type (0b1001 = available 64-bit tss)
        low.set_bits(40..44, 0b1001);

        let mut high = 0;
        high.set_bits(0..32, ptr.get_bits(32..64));

        Descriptor::SystemSegment(low, high)
    }
}

bitflags! {
    pub struct DescriptorFlags: u64 {
        /// Set by the processor if this segment has been accessed. Only cleared by software.
        /// _Setting_ this bit in software prevents GDT writes on first use.
        const ACCESSED     = 1 << 40;
        /// For 32-bit data segments, sets the segment as writable. For 32-bit code segments,
        /// sets the segment as _readable_. In 64-bit mode, ignored for all segments.
        const WRITABLE     = 1 << 41;
        /// This flag must be set for code segments and unset for data segments.
        const EXECUTABLE   = 1 << 43;
        /// This flag must be set for user segments (in contrast to system segments).
        const USER_SEGMENT = 1 << 44;
        /// The DPL for this descriptor is Ring 3. In 64-bit mode, ignored for data segments.
        const DPL_RING_3   = 3 << 45;
        /// Must be set for any segment, causes a segment not present exception if not set.
        const PRESENT      = 1 << 47;
        /// Must be set for 64-bit code segments, unset otherwise.
        const LONG_MODE    = 1 << 53;
        /// Limit field is scaled by 4096 bytes. In 64-bit mode, ignored for all segments.
        const GRANULARITY  = 1 << 55;
        /// Bits `0..=15` of the limit field (ignored in 64-bit mode)
        const LIMIT_0_15   = 0xFFFF;
        /// Bits `16..=19` of the limit field (ignored in 64-bit mode)
        const LIMIT_16_19  = 0xF << 48;
    }
}

impl DescriptorFlags {
    // Flags that we set for all our default segments
    const COMMON: Self = Self::from_bits_truncate(
        Self::USER_SEGMENT.bits()
            | Self::PRESENT.bits()
            | Self::WRITABLE.bits()
            | Self::ACCESSED.bits()
            | Self::LIMIT_0_15.bits()
            | Self::LIMIT_16_19.bits()
            | Self::GRANULARITY.bits(),
    );

    /// A 64-bit kernel code segment
    pub const KERNEL_CODE64: Self = Self::from_bits_truncate(
        Self::COMMON.bits() | Self::EXECUTABLE.bits() | Self::LONG_MODE.bits(),
    );
}
