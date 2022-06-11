use core::arch::asm;

use super::{addr::VirtualAddress, segmentation::SegmentSelector};

const NUMBER_OF_PRIVILEGE_LEVELS: usize = 3;
const NUMBER_OF_INTERRUPT_STACKS: usize = 7;

/// In 64-bit mode the TSS holds information that is not directly related to the task-switch
/// mechanism, but is used for finding kernel level stack if interrupts arrive while in kernel
/// mode.
///
/// Since the TSS uses the segmentation system (for historical reasons). Instead of loading the
/// table directly, we need to add a new segment descriptor to the Global Descriptor Table (GDT).
/// Then we can load our TSS invoking the ltr instruction with the respective GDT index.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub(crate) struct TaskStateSegment {
    reserved_1: u32,
    // The Privilege Stack Table is used by the CPU when the privilege level changes.
    //
    // Example:
    // If an exception occurs while the CPU is in user mode (privilege level 3), the CPU normally
    // switches to kernel mode (privilege level 0) before invoking the exception handler. In that
    // case, the CPU would switch to the 0th stack in the Privilege Stack Table (since 0 is the
    // target privilege level).
    privilege_stack_table: [VirtualAddress; NUMBER_OF_PRIVILEGE_LEVELS],
    reserved_2: u64,
    // The x86_64 architecture is able to switch to a predefined, known-good stack when an
    // exception occurs. This switch happens at hardware level, so it can be performed before the
    // CPU pushes the exception stack frame.
    // The switching mechanism is implemented as an Interrupt Stack Table (IST). The IST is a table
    // of 7 pointers to known-good stacks.
    pub(crate) interrupt_stack_table: [VirtualAddress; NUMBER_OF_INTERRUPT_STACKS],
    reserved_3: u64,
    reserved_4: u16,
    // TODO: Figure out what this is.
    iomap_base_address: u16,
}

impl TaskStateSegment {
    pub(crate) fn new() -> TaskStateSegment {
        TaskStateSegment {
            privilege_stack_table: [VirtualAddress::zero(); NUMBER_OF_PRIVILEGE_LEVELS],
            interrupt_stack_table: [VirtualAddress::zero(); NUMBER_OF_INTERRUPT_STACKS],
            iomap_base_address: 0,
            reserved_1: 0,
            reserved_2: 0,
            reserved_3: 0,
            reserved_4: 0,
        }
    }
}

/// Load the task state register using the `ltr` instruction.
///
/// ## Safety
///
/// This function is unsafe because the caller must ensure that the given
/// `SegmentSelector` points to a valid TSS entry in the GDT and that loading
/// this TSS is safe.
pub unsafe fn load_task_state_segment(sel: SegmentSelector) {
    asm!("ltr {0:x}", in(reg) sel.0, options(nomem, nostack, preserves_flags));
}
