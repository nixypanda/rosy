//! Initialize the Interrupt Descriptor Table bi setting various interrupt handlers.
use lazy_static::lazy_static;

use crate::{
    error, errorln,
    gdt::INTERRUPT_STACK_TABLE_INDEX_DOUBLE_FAULT,
    keyboard,
    pic8258::ChainedPics,
    utils::halt_loop,
    x86_64::{
        idt::{ExceptionStackFrame, InterruptDescriptorTable, PageFaultErrorCode},
        instructions::read_control_register_2,
        port::Port,
    },
};

/// Offset of the primary PIC in the PIC chain.
pub const PIC_1_OFFSET: u8 = 32;
/// Offset of the secondary PIC in the PIC chain.
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

const PS_2_CONTROLLER_PORT: u16 = 0x60;

/// Loads the IDT into the CPU.
pub fn init() {
    INTERRUPT_DESCRIPTOR_TABLE.load();
}

lazy_static! {
    /// The Interrupt Descriptor Table.
    ///
    /// Thi has the following handlers setup for following interrupts:
    /// * Breakpoint - Just prints the message along with the [`ExceptionStackFrame`]
    /// * Double Fault - Just prints the message along with the [`ExceptionStackFrame`] and then
    /// loops indefinitely.
    /// * Page Fault - Prints the message along with the [`ExceptionStackFrame`] along with the
    /// [`VirtualAddress`] that caused the page fault. Afterwards it just loops indefinitely.
    /// * Timer Interrupt - Notifyes the [`PROGRAMABLE_INTERRUPT_CONTROLERS`] that it is the end of
    /// interrupt and nothing else. i.e. it effectively does nothing.
    /// * Keyboard Interrupt - Makes use of the Colemak keypoard layout configuration to print the
    /// keycode to the screen. It prints the defult keycode if the key is not printable.
    pub static ref INTERRUPT_DESCRIPTOR_TABLE: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.set_breakpoint_handler(breakpoint_handler);
        unsafe {
            idt.set_double_fault_handler(double_fault_handler)
                .set_stack_index(INTERRUPT_STACK_TABLE_INDEX_DOUBLE_FAULT);
        }
        idt.set_page_fault_handler(page_fault_handler);
        idt.set_interrupt_handler(InterruptIndex::Timer.as_u8(), timer_interrupt_handler);
        idt.set_interrupt_handler(InterruptIndex::Keyboard.as_u8(), keyboard_interrupt_handler);
        idt
    };
}

lazy_static! {
    /// Sets up PICs at offset 32 and 40.
    pub static ref PROGRAMABLE_INTERRUPT_CONTROLERS: spin::Mutex<ChainedPics> =
        spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
}

// Exception Handlers

extern "x86-interrupt" fn breakpoint_handler(stack_frame: ExceptionStackFrame) {
    errorln!("EXCEPTION: BREAKPOINT ERROR\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: ExceptionStackFrame,
    _error_code: u64,
) -> ! {
    errorln!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    halt_loop();
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: ExceptionStackFrame,
    error_code: PageFaultErrorCode,
) {
    let responsible_virtual_address = read_control_register_2();

    errorln!("EXCEPTION: PAGE FAULT");
    errorln!("EXCEPTION: PAGE FAULT: Error Code: {:?}", error_code);
    errorln!(
        "EXCEPTION: PAGE FAULT: Virtual address responsible {:?}",
        responsible_virtual_address
    );
    errorln!("EXCEPTION: PAGE FAULT: Stack Frame\n{:#?}", stack_frame);
    halt_loop();
}

// Hardware PIC Interrupt Handlers

/// Hardware PIC Intrerrupt Handler offsets
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: ExceptionStackFrame) {
    unsafe {
        PROGRAMABLE_INTERRUPT_CONTROLERS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: ExceptionStackFrame) {
    let port = Port::new(PS_2_CONTROLLER_PORT);
    let scancode: u8 = unsafe { port.read() };
    keyboard::add_scancode(scancode);

    unsafe {
        PROGRAMABLE_INTERRUPT_CONTROLERS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

// utilities

/// Cause a page fault to occur
pub fn invoke_page_fault_exception() {
    unsafe {
        *(0xdeadbeef as *mut u64) = 42;
    };
}

// Tests

/// Cause a stack overflow to occur
#[allow(unconditional_recursion)]
pub fn stack_overflow() {
    stack_overflow();
}

#[test_case]
fn test_breakpoint_exception() {
    use crate::x86_64::interrupts::invoke_breakpoint_exception;
    // Execution continues => Breakpoint handler is working
    invoke_breakpoint_exception();
}
