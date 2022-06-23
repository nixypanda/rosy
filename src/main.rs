#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rosy::{
    async_runtime::{Executor, Task},
    keyboard, print, println,
    utils::halt_loop,
};

entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    rosy::init(boot_info);

    println!("Hello World!");

    execute_async_tasks();

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    halt_loop();
}

async fn async_number() -> usize {
    42
}

async fn perform_async_printing() {
    let number = async_number().await;
    println!("async number {}", number);
}

fn execute_async_tasks() {
    let mut executor = Executor::new();
    executor.spawn(Task::new(perform_async_printing()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);

    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rosy::test_panic_handler(info);
}
