#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rosy::async_runtime::{Executor, Task};

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    rosy::init(boot_info);

    test_main();
    loop {}
}

#[test_case]
fn test_async_task_execution() {
    let mut executor = Executor::new();
    executor.spawn(Task::new(perform_async_printing()));
    // We only run ready tasks so as to make sure that the executor stops
    // There should be a better way to do this kind of testing.
    executor.run_ready_tasks();
}

async fn async_number() -> usize {
    42
}

async fn perform_async_printing() {
    let number = async_number().await;
    assert_eq!(42, number);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rosy::test_panic_handler(info)
}
