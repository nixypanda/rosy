//! A small OS created in Rust. Currently it can
//!
//! - Print to the screen
//! - Handle Breakpoint Exception (INT3)
//! - Handle Page Fault Exception (PF) [does not do anything special yet, just prints the error]
//! - Handle Double Fault Exception (DF) [does not do anything special yet, just prints the error]
//! - Handle Timer interrupts
//! - Handle Keyboard interrupts (Has support for even Colemak)
//! - Can translate Virtual addresses to Physical addresses using offset based paging.

#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![feature(generators)]
#![feature(generator_trait)]

extern crate alloc;

pub mod allocation;
pub mod allocator;
pub mod async_runtime;
pub mod gdt;
pub mod interrupt;
pub mod keyboard;
pub mod memory;
pub mod pic8258;
pub mod ps2_keyboard_decoder;
pub mod screen_printing;
pub mod serial;
pub mod utils;
pub mod vga;
pub mod x86_64;

use bootloader::BootInfo;
use core::ops::Fn;
use core::panic::PanicInfo;

use x86_64::port::Port;

#[cfg(test)]
use bootloader::entry_point;

#[cfg(test)]
entry_point!(test_kernel_main);

pub fn init(boot_info: &'static BootInfo) {
    gdt::init();
    interrupt::init();
    unsafe {
        interrupt::PROGRAMABLE_INTERRUPT_CONTROLERS
            .lock()
            .initialize()
    };
    x86_64::interrupts::enable();
    memory::init(boot_info);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_error!("[Failed]");
    serial_println!();
    serial_error!("Error: {}", info);
    serial_println!();
    exit_qemu(QemuExitCode::Failed);

    loop {}
}

#[cfg(test)]
#[no_mangle]
pub fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    use utils::halt_loop;

    init(boot_info);
    test_main();

    halt_loop();
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
        serial_success!("[ok]");
        serial_println!();
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
