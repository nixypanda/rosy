//! High level printing helpers to send info over the serial interface

use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

use crate::x86_64::interrupts::execute_without_interrupts;

lazy_static! {
    #[doc(hidden)]
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;

    // Execute without interrupts disables interrupts while executing a piece of code. We use it to
    // ensure that no interrupt cannot occur as long as the Mutex is locked.
    // Hardware interrupts can occur asynchronously while the Mutex is locked. In that situation
    // WRITER is locked the interrupt handler waits on the Mutex to be unlocked. But this never
    // happens as the `_start` is waiting on the interrupt handler to finish.
    execute_without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (
        $crate::serial_print!(concat!($fmt, "\n"), $($arg)*)
    );
}

/// Prints to the host through the serial interface using green coloring
#[macro_export]
macro_rules! serial_success {
    ($fmt:expr) => {
        $crate::serial_print!(concat!("\x1b[92m", $fmt, "\x1b[0m"))
    };
    ($fmt:expr, $($arg:tt)*) => (
        $crate::serial_print!(concat!("\x1b[92m", $fmt, "\x1b[0m"), $($arg)*)
    );
}

/// Prints to the host through the serial interface using red coloring
#[macro_export]
macro_rules! serial_error {
    ($fmt:expr) => {
        $crate::serial_print!(concat!("\x1b[91m", $fmt, "\x1b[0m"))
    };
    ($fmt:expr, $($arg:tt)*) => (
        $crate::serial_print!(concat!("\x1b[91m", $fmt, "\x1b[0m"), $($arg)*)
    );
}
