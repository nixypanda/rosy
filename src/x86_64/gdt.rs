//! Provides types for the Global Descriptor Table and its entries.

use core::arch::asm;

use super::{
    address::VirtualAddress,
    descriptor::{
        Descriptor, DescriptorFlags, DescriptorTablePointer, BYTES_IN_SYSTEM_SEGMENT_DESCRIPTOR,
        BYTES_IN_USER_SEGMENT_DESCRIPTOR,
    },
    privilege_level::PrivilegeLevel,
    segmentation::SegmentSelector,
};

const GDT_ENTRY_COUNT: usize = 8;
const BYTES_IN_GDT_ENTRY: usize = 8;

/// The Global Descriptor Table is a construct used by the x86 processor to configure segmented
/// virtual memory.
///
/// It came long before paging was added to the architecture and as such is a
/// legacy piece of configuration. Very few protected-mode operating systems (which most are) use
/// segmentation. Instead paging is favoured. However, because the x86 processor retains backwards
/// compatibility, in order to use paging, basic segmentation must still be configured.
///
/// The GDT is majorly used for two things:
/// 1. The GDT also contains a TSS (Task State Segment) entry which has to be configured for task
/// switching. We need GDT to load TSS.
/// 2. Switching between kernel space and user space.
#[derive(Debug, Clone)]
pub struct GlobalDescriptorTable {
    table: [u64; GDT_ENTRY_COUNT],
    next_free: usize,
}

impl GlobalDescriptorTable {
    /// Creates an empty GDT.
    pub fn new() -> GlobalDescriptorTable {
        GlobalDescriptorTable {
            table: [0; GDT_ENTRY_COUNT],
            next_free: 1,
        }
    }

    /// Loads the GDT in the CPU using the `lgdt` instruction. This does **not** alter any of the
    /// segment registers; you **must** (re)load them yourself using [the appropriate
    /// functions]
    pub fn load(&self) {
        use core::mem::size_of;

        let ptr = DescriptorTablePointer::new(
            VirtualAddress::new(self.table.as_ptr() as u64),
            (self.next_free * size_of::<u64>() - 1) as u16,
        );

        unsafe {
            load_global_descriptor_table(&ptr);
        };
    }

    /// Adds the given segment descriptor to the GDT, returning the segment selector.
    ///
    /// Panics if the GDT has no free entries left.
    pub fn add_entry(&mut self, entry: Descriptor) -> SegmentSelector {
        let index = match entry {
            Descriptor::UserSegment(value) => {
                let size = BYTES_IN_USER_SEGMENT_DESCRIPTOR / BYTES_IN_GDT_ENTRY;
                if self.next_free > self.table.len().saturating_sub(size) {
                    panic!("GDT full");
                } else {
                    self.push(value)
                }
            }
            Descriptor::SystemSegment(low, high) => {
                let size = BYTES_IN_SYSTEM_SEGMENT_DESCRIPTOR / BYTES_IN_GDT_ENTRY;
                if self.next_free > self.table.len().saturating_sub(size) {
                    panic!("GDT full");
                } else {
                    let index_low = self.push(low);
                    self.push(high);
                    index_low
                }
            }
        };
        let privilege_level = match entry {
            Descriptor::UserSegment(value) => {
                if DescriptorFlags::from_bits_truncate(value).contains(DescriptorFlags::DPL_RING_3)
                {
                    PrivilegeLevel::Ring3
                } else {
                    PrivilegeLevel::Ring0
                }
            }
            Descriptor::SystemSegment(_, _) => PrivilegeLevel::Ring0,
        };

        SegmentSelector::new(index as u16, privilege_level)
    }

    fn push(&mut self, value: u64) -> usize {
        let index = self.next_free;
        self.table[index] = value;
        self.next_free += 1;
        index
    }
}

/// Loads the GlobalDesriptorTable by calling the `lgdt` instruction.
///
/// ## Safety
///
/// This function is unsafe because the caller must ensure that the given `DescriptorTablePointer`
/// points to a valid GDT and that loading this GDT is safe.
unsafe fn load_global_descriptor_table(gdt: &DescriptorTablePointer) {
    asm!("lgdt [{}]", in(reg) gdt, options(readonly, nostack, preserves_flags));
}
