use core::{
    arch::asm,
    ops::{Index, Range},
};

use bit_field::BitField;
use bitflags::bitflags;

use super::{
    address::VirtualAddress,
    descriptor::DescriptorTablePointer,
    segmentation::{get_current_code_segment, SegmentSelector},
};

const DEFAULT_RESERVED: u32 = 0;
const IDT_SIZE: usize = 64;

const IDT_INDEX_BREAKPOINT_EXCEPTION: u8 = 3;
const IDT_INDEX_DOUBLE_FAULT_EXCEPTION: u8 = 8;
const IDT_INDEX_PAGE_FAULT_EXCEPTION: u8 = 14;

const NUMBER_OF_EXCEPTION_HANDLERS: u8 = 32;

const ENTRY_OPTIONS_IST_INDEX_BITS: Range<usize> = 0..3;

/// The harware calls the Interrupt Descriptor Table (IDT) to handle all the interrupts that can
/// occur. The hardware uses this table directly so we need to follow a predefined format.
///
/// We create an Interrupt Descriptor Table (IDT) with 16 entries. Ideally it has 256 entries.
/// When the entries are missing the CPU simply generates a double fault.
///
/// The 16 entries that we take care of here are:
/// - Divide by zero
/// - Debug
/// - Non maskable interrupt
/// - Breakpoint
/// - Overflow
/// - Bound range exceeded
/// - Invalid opcode
/// - device not available
/// - double fault
/// - coprocessor segment overrun
/// - invalid TSS
/// - segment not Present
/// - stack fault
/// - general protection fault
/// - page fault
/// - reserved
pub struct InterruptDescriptorTable([Entry; IDT_SIZE]);

/// Each entry in the Interrupt Descriptor Table (IDT) has the following structure.
///
/// | Type	| Name	                   | Description                                              |
/// | u16	| Function Pointer [0:15]  | The lower bits of the pointer to the handler function.   |
/// | u16	| GDT selector	           | Selector of a code segment in the GDT.                   |
/// | u16	| Options	(see below)    |                                                          |
/// | u16	| Function Pointer [16:31] | The middle bits of the pointer to the handler function.  |
/// | u32	| Function Pointer [32:63] | The remaining bits of the pointer to the handler function|
/// | u32	| Reserved                 |                                                          |
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct Entry {
    pointer_low: u16,
    gdt_selector: u16,
    pub options: EntryOptions,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

bitflags! {
    #[repr(transparent)]
    pub struct EntryOptions: u16 {
        // | 0-2   | Interrupt Stack Table Index | 0: Don’t switch stacks,                        |
        // |       |                             | 1-7: Switch to the n-th stack in the Interrupt |
        // |       |                             | Stack Table when this handler is called.       |
        // 3 - 7 are Reserved
        // When this bit is 0, interrupts are disabled when this handler is called.
        const INTERRUPTS_ENABLED = 1 << 8;
        // These bits must be set to one
        const BIT_9              = 1 << 9;
        const BIT_10             = 1 << 10;
        const BIT_11             = 1 << 11;
        const MUST_BE_ONE        = Self::BIT_9.bits | Self::BIT_10.bits | Self::BIT_11.bits;
        // | 13‑14 | Descriptor Privilege Level (DPL) | The minimal privilege level required |
        // |       |                                  | for calling this handler.            |
        const DPL_LOW            = 1 << 13;
        const DPL_HIGH           = 1 << 14;
        const DPL_MASK           = Self::DPL_LOW.bits | Self::DPL_HIGH.bits;
        // Says that the handler is present.
        const PRESENT            = 1 << 15;
    }
}

impl EntryOptions {
    fn minimal() -> Self {
        EntryOptions::MUST_BE_ONE
    }

    fn new() -> Self {
        let mut options = Self::minimal();
        options.set_present().disable_interrupts();
        options
    }

    fn set_present(&mut self) -> &mut Self {
        self.set(EntryOptions::PRESENT, true);
        self
    }

    fn disable_interrupts(&mut self) -> &mut Self {
        self.set(EntryOptions::INTERRUPTS_ENABLED, false);
        self
    }

    /// Assigns a Interrupt Stack Table (IST) stack to this handler. The CPU will then always
    /// switch to the specified stack before the handler is invoked. This allows kernels to
    /// recover from corrupt stack pointers (e.g., on kernel stack overflow).
    ///
    /// An IST stack is specified by an IST index between 0 and 6 (inclusive). Using the same
    /// stack for multiple interrupts can be dangerous when nested interrupts are possible.
    ///
    /// This function panics if the index is not in the range 0..7.
    ///
    /// ## Safety
    ///
    /// This function is unsafe because the caller must ensure that the passed stack index is
    /// valid and not used by other interrupts. Otherwise, memory safety violations are possible.
    pub unsafe fn set_stack_index(&mut self, index: u16) {
        // The hardware IST index starts at 1, but our software IST index
        // starts at 0. Therefore we need to add 1 here.
        self.bits.set_bits(ENTRY_OPTIONS_IST_INDEX_BITS, index + 1);
    }
}

impl Entry {
    fn new(gdt_selector: SegmentSelector, pointer_to_handler: u64) -> Self {
        Entry {
            pointer_low: pointer_to_handler as u16,
            pointer_middle: (pointer_to_handler >> 16) as u16,
            pointer_high: (pointer_to_handler >> 32) as u32,
            gdt_selector: gdt_selector.0,
            options: EntryOptions::new(),
            reserved: DEFAULT_RESERVED,
        }
    }

    fn missing() -> Entry {
        Entry {
            pointer_low: 0,
            pointer_middle: 0,
            pointer_high: 0,
            gdt_selector: 0,
            options: EntryOptions::new(),
            reserved: DEFAULT_RESERVED,
        }
    }
}

impl InterruptDescriptorTable {
    pub fn new() -> Self {
        InterruptDescriptorTable([Entry::missing(); IDT_SIZE])
    }

    fn set_handler(&mut self, index: u8, handler_func: HandlerFunc) {
        self.0[index as usize] = Entry::new(get_current_code_segment(), handler_func as u64);
    }

    /// A breakpoint (`#BP`) exception occurs when an `INT3` instruction is executed. The
    /// `INT3` is normally used by debug software to set instruction breakpoints by replacing
    ///
    /// The saved instruction pointer points to the byte after the `INT3` instruction.
    ///
    /// The vector number of the `#BP` exception is 3.
    pub fn set_breakpoint_handler(&mut self, handler_func: HandlerFunc) {
        self.set_handler(IDT_INDEX_BREAKPOINT_EXCEPTION, handler_func);
    }

    /// Double fault exception can occur when a second exception occurs during the handling of a
    /// prior (first) exception handler
    ///
    /// The following combinations result in a double fault:
    ///
    /// | First Exception           |	Second Exception         |
    /// |---------------------------|----------------------------|
    /// | Divide-by-zero,           |  Invalid TSS,              |                 
    /// | Invalid TSS,              |  Segment Not Present,      |                 
    /// | Segment Not Present,      |  Stack-Segment Fault,      |                 
    /// | Stack-Segment Fault,      |  General Protection Fault  |                   
    /// | General Protection Fault  |                            |
    /// |---------------------------|----------------------------|
    /// | Page Fault	            | Page Fault,                |
    /// |                           |  Invalid TSS,              |                        
    /// |                           |  Segment Not Present,      |                        
    /// |                           |  Stack-Segment Fault,      |                           
    /// |                           |  General Protection Fault  |                          
    ///
    /// If a third interrupting event occurs while transferring control to the `#DF` handler, the
    /// processor shuts down.
    #[allow(unaligned_references)]
    pub fn set_double_fault_handler(
        &mut self,
        handler_func: DoubleFaultHandlerFunc,
    ) -> &mut EntryOptions {
        let entry = Entry::new(get_current_code_segment(), handler_func as u64);
        self.0[IDT_INDEX_DOUBLE_FAULT_EXCEPTION as usize] = entry;

        &mut self.0[IDT_INDEX_DOUBLE_FAULT_EXCEPTION as usize].options
    }

    pub fn set_page_fault_handler(&mut self, handler_func: PageFaultHandlerFunc) {
        self.0[IDT_INDEX_PAGE_FAULT_EXCEPTION as usize] =
            Entry::new(get_current_code_segment(), handler_func as u64);
    }

    pub fn set_hardware_interrupt(&mut self, index: u8, handler_func: HandlerFunc) {
        if index < NUMBER_OF_EXCEPTION_HANDLERS {
            panic!(
                "Can't add hardware interrupt handler at inder {}. First {} indicies are reserved
                 for specific handlers",
                index,
                NUMBER_OF_EXCEPTION_HANDLERS - 1
            );
        }
        self.set_handler(index, handler_func);
    }

    pub fn load(&self) {
        use core::mem::size_of;

        let ptr = DescriptorTablePointer::new(
            VirtualAddress::new(self as *const _ as u64),
            (size_of::<Self>() - 1) as u16,
        );

        unsafe { load_iterrupt_descriptor_table(&ptr) };
    }
}

/// Loads the InterruptDescriptorTable by calling the lidt instruction
///
/// ## Safety
///
/// This function is unsafe because the caller must ensure that the given `DescriptorTablePointer`
/// points to a valid IDT and that loading this IDT is safe.
unsafe fn load_iterrupt_descriptor_table(idt: &DescriptorTablePointer) {
    // https://www.felixcloutier.com/x86/lgdt:lidt
    asm!("lidt [{}]", in(reg) idt, options(nostack));
}

/// Represents the interrupt stack frame pushed by the CPU on an interrupt or exception entry.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ExceptionStackFrame {
    /// This value points to the instruction that should be executed when the interrupt
    /// handler returns. For most interrupts, this value points to the instruction immediately
    /// following the last executed instruction.
    instruction_pointer: VirtualAddress,
    /// The code segment selector, padded with zeros.
    code_segment: u64,
    /// The value of the `rflags` register at the time of the interrupt (or before the interrupt
    /// handler was called).
    cpu_flags: u64,
    /// The stack pointer at the time of the interrupt.
    stack_pointer: VirtualAddress,
    /// The stack segment descriptor at the time of the interrupt
    stack_segment: u64,
}

/// Why use x86-interrupt calling convention?
/// - aware that the arguments lie on the stack
/// - uses iretq instruction to return instead of normal ret
/// - handles error codes if proper types are supplied. Error codes can change stack alignment,
/// this calling convention takes care of all that complexity for us
///
/// Given that Entry has pointers to actual handlers we can use different types in the HandlerFunc
type HandlerFunc = extern "x86-interrupt" fn(ExceptionStackFrame);

type DoubleFaultHandlerFunc = extern "x86-interrupt" fn(ExceptionStackFrame, u64) -> !;

type PageFaultHandlerFunc = extern "x86-interrupt" fn(ExceptionStackFrame, PageFaultErrorCode);

bitflags! {
    #[repr(transparent)]
    pub struct PageFaultErrorCode: u64 {
        // When set, the page fault was caused by a page-protection violation. When not set, it was
        // caused by a non-present page.
        const PROTECTION_VIOLATION     = 1 << 0;
        // When set, the page fault was caused by a write access. When not set, it was caused by a
        // read access. Does not necessarily indicate if this was caused by a read or a write
        // instruction.
        const CAUSED_BY_WRITE          = 1 << 1;
        // When set, the page fault was caused while CPL = 3. Else the fault was caused in
        // supervisor mode (CPL 0, 1, or 2). This does not necessarily mean that the page fault was
        // a privilege violation.
        const CAUSED_BY_USER           = 1 << 2;
        // hen set, one or more page directory entries contain reserved bits which are set to 1.
        // This only applies when the PSE or PAE flags in CR4 are set to 1
        const MALFORMED_TABLE          = 1 << 3;
        // When set, the page fault was caused by an instruction fetch. This only applies when the
        // No-Execute bit is supported and enabled.
        const INSTRUCTION_FETCH        = 1 << 4;
        // When set, the page fault was caused by a protection-key violation. The PKRU register
        // (for user-mode accesses) or PKRS MSR (for supervisor-mode accesses) specifies the
        // protection key rights.
        const PROTECTION_KEY_VIOLATION = 1 << 5;
        // When set, the page fault was caused by a shadow stack access.
        const CAUSED_BY_SHADOW_STACK   = 1 << 6;
    }
}
