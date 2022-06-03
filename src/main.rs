#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::arch::asm;
use core::panic::PanicInfo;
use rosy::{print, println};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World!");

    rosy::init();

    invoke_breakpoint_exception();

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    loop {}
}

fn invoke_breakpoint_exception() {
    // Cause a breakpoint exception by invoking the `int3` instruction.
    // https://en.wikipedia.org/wiki/INT_%28x86_instruction%29
    unsafe { asm!("int3", options(nomem, nostack)) }
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
