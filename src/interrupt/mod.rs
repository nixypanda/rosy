use lazy_static::lazy_static;

use crate::{print, println};

use self::idt::ExceptionStackFrame;

mod idt;

lazy_static! {
    pub static ref INTERRUPT_DESCRIPTOR_TABLE: idt::InterruptDescriptorTable = {
        let mut idt = idt::InterruptDescriptorTable::new();
        idt.set_breakpoint_handler(breakpoint_handler);
        idt
    };
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: ExceptionStackFrame) {
    println!("EXCEPTION: BREAKPOINT ERROR\n{:#?}", stack_frame);
}

pub fn init() {
    INTERRUPT_DESCRIPTOR_TABLE.load();
}
