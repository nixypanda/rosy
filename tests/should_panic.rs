#![no_std]
#![no_main]

use core::panic::PanicInfo;
use rosy::{exit_qemu, serial_error, serial_print, serial_println, serial_success, QemuExitCode};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_error!("[test did not panic]");
    serial_println!();
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

fn should_fail() {
    serial_print!("should_panic::should_fail...\t");
    assert_eq!(0, 1);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_success!("[ok]");
    serial_println!();
    exit_qemu(QemuExitCode::Success);
    loop {}
}
