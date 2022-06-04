use core::arch::asm;

use lazy_static::lazy_static;

use crate::{print, println};

use self::idt::ExceptionStackFrame;

mod idt;

lazy_static! {
    pub static ref INTERRUPT_DESCRIPTOR_TABLE: idt::InterruptDescriptorTable = {
        let mut idt = idt::InterruptDescriptorTable::new();
        idt.set_breakpoint_handler(breakpoint_handler);
        idt.set_double_fault_handler(double_fault_handler);
        idt
    };
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
