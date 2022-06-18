#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use lazy_static::lazy_static;
use rosy::{
    exit_qemu, serial_print, serial_println, serial_success,
    x86_64::idt::{ExceptionStackFrame, InterruptDescriptorTable},
    QemuExitCode,
};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!();
    serial_print!("stack_overflow::stack_overflow...\t");

    rosy::gdt::init();
    init_test_idt();

    // trigger a stack overflow
    stack_overflow();

    panic!("Execution continued after stack overflow");
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow(); // for each recursion, the return address is pushed
    volatile::Volatile::new(0).read(); // prevent tail recursion optimizations
}

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.set_double_fault_handler(test_double_fault_handler)
                .set_stack_index(rosy::gdt::INTERRUPT_STACK_TABLE_INDEX_DOUBLE_FAULT);
        }

        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: ExceptionStackFrame,
    _error_code: u64,
) -> ! {
    serial_success!("[ok]");
    serial_println!();
    serial_println!();
    exit_qemu(QemuExitCode::Success);
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rosy::test_panic_handler(info)
}
