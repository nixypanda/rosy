#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rosy::{print, println};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World!");

    rosy::init();

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    loop {
        print!("+");
        for _ in 0..10000 {
            // Do nothing
        }
    }
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
