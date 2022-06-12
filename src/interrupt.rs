use core::arch::asm;

use lazy_static::lazy_static;

use crate::{
    gdt::INTERRUPT_STACK_TABLE_INDEX_DOUBLE_FAULT,
    pic8258::ChainedPics,
    print, println,
    x86_64::idt::{ExceptionStackFrame, InterruptDescriptorTable},
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

lazy_static! {
    pub static ref INTERRUPT_DESCRIPTOR_TABLE: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.set_breakpoint_handler(breakpoint_handler);
        unsafe {
            idt.set_double_fault_handler(double_fault_handler)
                .set_stack_index(INTERRUPT_STACK_TABLE_INDEX_DOUBLE_FAULT);
        }
        idt.set_hardware_interrupt(InterruptIndex::Timer.as_u8(), timer_interrupt_handler);
        idt
    };
}

lazy_static! {
    pub static ref PROGRAMABLE_INTERRUPT_CONTROLERS: spin::Mutex<ChainedPics> =
        spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: ExceptionStackFrame) {
    println!("EXCEPTION: BREAKPOINT ERROR\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: ExceptionStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: ExceptionStackFrame) {
    print!(".");

    unsafe {
        PROGRAMABLE_INTERRUPT_CONTROLERS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

pub fn init() {
    INTERRUPT_DESCRIPTOR_TABLE.load();
}

pub fn invoke_breakpoint_exception() {
    // Cause a breakpoint exception by invoking the `int3` instruction.
    // https://en.wikipedia.org/wiki/INT_%28x86_instruction%29
    unsafe { asm!("int3", options(nomem, nostack)) }
}

pub fn invoke_page_fault_exception() {
    unsafe {
        *(0xdeadbeef as *mut u64) = 42;
    };
}

#[allow(unconditional_recursion)]
pub fn stack_overflow() {
    stack_overflow();
}

#[test_case]
fn test_breakpoint_exception() {
    // Execution continues => Breakpoint handler is working
    invoke_breakpoint_exception();
}
