#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rosy::{
    interrupt::{invoke_breakpoint_exception, invoke_page_fault_exception, stack_overflow},
    print, println,
};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World!");

    rosy::init();

    invoke_breakpoint_exception();
    println!("It did not crash!");

    // invoke_page_fault_exception();
    stack_overflow();

    #[cfg(test)]
    test_main();

    loop {}
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
