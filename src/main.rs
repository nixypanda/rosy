#![no_std]
#![no_main]

mod vga_buffer;

use core::fmt::Write;
use core::panic::PanicInfo;

use vga_buffer::WRITER;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut writer = WRITER.lock();

    writer.write_string("Hello ");
    writer.write_string("Wörld!");

    write!(WRITER.lock(), "Printing from write macro {}", 1236).unwrap();

    loop {}
}
