use core::arch::asm;

use bit_field::BitField;

use super::addr::VirtualAddress;

const DEFAULT_RESERVED: u32 = 0;

const IDT_INDEX_BREAKPOINT_EXCEPTION: u8 = 3;
const IDT_INDEX_DOUBLE_FAULT_EXCEPTION: u8 = 8;

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
pub struct InterruptDescriptorTable([Entry; 16]);

/// Why use x86-interrupt calling convention?
/// - aware that the arguments lie on the stack
/// - uses iretq instruction to return instead of normal ret
/// - handles error codes if proper types are supplied. Error codes can change stack alignment,
/// this calling convention takes care of all that complexity for us
///
/// Given that Entry has pointers to actual handlers we can use different types in the HandlerFunc
type HandlerFunc = extern "x86-interrupt" fn(ExceptionStackFrame);

type DoubleFaultHandlerFunc = extern "x86-interrupt" fn(ExceptionStackFrame, u64) -> !;

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
    options: EntryOptions,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

/// | Bits  |	Name                           |     Description                                  |
/// | 0-2   | Interrupt Stack Table Index      | 0: Don’t switch stacks,                          |
/// |       |                                  | 1-7: Switch to the n-th stack in the Interrupt   |
/// |       |                                  | Stack Table when this handler is called.         |
/// | 3-7   | Reserved                         |                                                  |
/// | 8     | 0: Interrupt Gate,               | If this bit is 0, interrupts are disabled when   |
/// |       | 1: Trap Gate                     | this handler is called.                          |
/// | 9-11  | must be one                      |                                                  |
/// | 12	| must be zero                     |                                                  |
/// | 13‑14	| Descriptor Privilege Level (DPL) | The minimal privilege level required for calling |
/// |       |                                  | this handler.                                    |
/// | 15	| Present                          |                                                  |
#[derive(Debug, Clone, Copy)]
struct EntryOptions(u16);

impl EntryOptions {
    fn minimal() -> Self {
        let mut options = 0;
        options.set_bits(9..12, 0b111); // 'must-be-one' bits
        EntryOptions(options)
    }

    fn new() -> Self {
        let mut options = Self::minimal();
        options.set_present(true).disable_interrupts(true);
        options
    }

    pub fn set_present(&mut self, present: bool) -> &mut Self {
        self.0.set_bit(15, present);
        self
    }

    pub fn disable_interrupts(&mut self, disable: bool) -> &mut Self {
        self.0.set_bit(8, !disable);
        self
    }
}

impl Entry {
    fn new(gdt_selector: u16, pointer_to_handler: u64) -> Self {
        Entry {
            pointer_low: pointer_to_handler as u16,
            pointer_middle: (pointer_to_handler >> 16) as u16,
            pointer_high: (pointer_to_handler >> 32) as u32,
            gdt_selector,
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
        InterruptDescriptorTable([Entry::missing(); 16])
    }

    fn set_handler(&mut self, index: u8, handler_func: HandlerFunc) {
        self.0[index as usize] = Entry::new(get_current_code_segment().0, handler_func as u64);
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
    pub fn set_double_fault_handler(&mut self, handler_func: DoubleFaultHandlerFunc) {
        self.0[IDT_INDEX_DOUBLE_FAULT_EXCEPTION as usize] =
            Entry::new(get_current_code_segment(), handler_func as u64);
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

fn get_current_code_segment() -> u16 {
    let segment: u16;
    unsafe {
        asm!("mov {0:x}, cs", out(reg) segment, options(nomem, nostack, preserves_flags));
    }
    segment
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

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct DescriptorTablePointer {
    /// Size of the DT.
    limit: u16,
    /// Pointer to the memory region containing the DT.
    base: VirtualAddress,
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
