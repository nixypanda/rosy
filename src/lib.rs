#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]

pub mod gdt;
pub mod interrupt;
pub mod keyboard;
pub mod pic8258;
pub mod serial;
pub mod vga_buffer;
pub mod x86_64;

use core::ops::Fn;
use core::panic::PanicInfo;

use x86_64::port::Port;

pub fn init() {
    crate::gdt::init();
    crate::interrupt::init();
    unsafe {
        interrupt::PROGRAMABLE_INTERRUPT_CONTROLERS
            .lock()
            .initialize()
    };
    crate::x86_64::interrupts::enable();
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[Failed]");
    serial_println!("Error: {}", info);
    exit_qemu(QemuExitCode::Failed);

    loop {}
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    init();
    test_main();

    loop {}
}

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!();
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    serial_println!();
    exit_qemu(QemuExitCode::Success);
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    let port: Port<u32> = Port::new(0xf4);
    unsafe {
        port.write(exit_code as u32);
    }
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
