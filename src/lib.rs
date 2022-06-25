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
pub mod shell;
pub mod utils;
pub mod vga;
pub mod x86_64;

use async_runtime::{Executor, Task};
use bootloader::BootInfo;
use core::ops::Fn;
use core::panic::PanicInfo;
use shell::Shell;

use x86_64::port::Port;

#[cfg(test)]
use bootloader::entry_point;

#[cfg(test)]
entry_point!(test_kernel_main);

const ISA_EXIT_DEVICE_IOBASE: u16 = 0xf4;

/// Initialize the OS
///
/// * Setup Global Descriptor Table
/// * Setup Interrupt Descriptor Table
/// * Setup Programable Interrupt Controllers
/// * Enable interrupts
/// * Setup offset based memory mapping
/// * Setup heap allocator
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

/// Initialize async jobs
///
/// Note: We need to call `run` on [`Executor`] seprately to start there tasks
///
/// * Setup shell task
pub fn init_async_tasks<'a>(executor: &mut Executor<'a>, shell: &'a mut Shell) {
    executor.spawn(Task::new(shell.run()))
}

// Testing related stuff
// Stuff that is not marked with `#[cfg(test)]` is used by integration tests

/// Panic handler that prints information to the serial interface.
///
/// This is helpful for testing as we can view this information on the host machine. It also exits
/// qemu with a faild status code.
///
/// This is just a normal function the user needs to register it in order to use it.
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

/// Trait that effectively uses aspect oriented programming to add some information to a test when
/// we run it.
///
/// We declare a method `run` in this trait then implement this trait for all types `T` that
/// implement the `Fn` trait (i.e all functions)
pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    /// Print a the test name followed by `[ok]` (in green coloring) if it succeeds.
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_success!("[ok]");
        serial_println!();
    }
}

/// A custom test runner.
///
/// When we use `custom_test_frameworks` feature, it collects all the functions marked with
/// `#[test_case]` and provides them to a custom test runner that we have to register (this is that
/// function).
/// It makes use of the [`Testable`] trait to call `run` on each of the tests. After all is done it
/// exits qemu with success status code.
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!();
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    serial_println!();
    exit_qemu(QemuExitCode::Success);
}

/// Tests specific panic handler. It is invoked when we run unit tests.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

/// Exit codes that will be used when we quit qemu using the `isa-debug-device`.
///
/// The values don't matter we just don't want to use codes that qemu already uses.
/// After the transformation the code will become `(value << 1) | 1`.
/// Check the `Cargo.toml` `package.metadata.bootimage.test-success-exit-code`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    /// Success exit code: (0b10000 << 1) | 1 = 0b100001 (or 33) (or 0x21)
    Success = 0x10,
    /// Failure exit code: (0b11000 << 1) | 1 = 0b110001 (or 49) (or 0x31)
    Failed = 0x11,
}

/// Uses port-mapped I/O to communicate.
///
/// It is intended to be used with the `isa-debug-exit` device to exit qemu.
pub fn exit_qemu(exit_code: QemuExitCode) {
    let port: Port<u32> = Port::new(ISA_EXIT_DEVICE_IOBASE);
    unsafe {
        port.write(exit_code as u32);
    }
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
