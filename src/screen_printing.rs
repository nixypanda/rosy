//! High level screen printing abstractions.
//!
//! Mostly just houses macros that use [`static@WRITER`] to perform printing operations on the
//! screen.

use core::fmt::{self, Write};
use lazy_static::lazy_static;

use crate::{
    vga::{ColorCode, Writer},
    x86_64::interrupts::execute_without_interrupts,
};

#[cfg(test)]
use crate::vga::ScreenChar;

lazy_static! {
    /// Global instance of [`Writer`].
    ///
    pub static ref WRITER: spin::Mutex<Writer> = spin::Mutex::new(Writer::default());
}

/// Use [`static@WRITER`] to write to the VGA buffer using default coloring with newline
#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => (print!("{}\n", format_args!($($arg)*)));
}

/// Use [`static@WRITER`] to write to the VGA buffer using default coloring without newline
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => (
        $crate::screen_printing::_print(
            *$crate::vga::DEFAULT_COLOR_CODE,
            format_args!($($arg)*)
        )
    );
}

/// Use [`static@WRITER`] to write to the VGA buffer using default coloring with newline
#[macro_export]
macro_rules! errorln {
    () => (error!("\n"));
    ($($arg:tt)*) => (error!("{}\n", format_args!($($arg)*)));
}

/// Use [`static@WRITER`] to write to the VGA buffer using default coloring without newline
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => (
        $crate::screen_printing::_print(
            *$crate::vga::ERROR_COLOR_CODE,
            format_args!($($arg)*)
        )
    );
}

/// Use [`static@WRITER`] to write to the VGA buffer using yellow coloring without newline
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => (
        $crate::screen_printing::_print(
            *$crate::vga::WARN_COLOR_CODE,
            format_args!($($arg)*)
        )
    );
}

#[doc(hidden)]
pub fn _print(color_code: ColorCode, args: fmt::Arguments) {
    // Execute without interrupts disables interrupts while executing a piece of code. We use it to
    // ensure that no interrupt cannot occur as long as the Mutex is locked.
    // Hardware interrupts can occur asynchronously while the Mutex is locked. In that situation
    // WRITER is locked the interrupt handler waits on the Mutex to be unlocked. But this never
    // happens as the `_start` is waiting on the interrupt handler to finish.
    execute_without_interrupts(|| {
        WRITER
            .lock()
            .with_color_code(color_code)
            .write_fmt(args)
            .unwrap();
    });
}

#[test_case]
fn test_println_macro_prints_one_line_without_panicking() {
    println!("This is onen line");
}

#[test_case]
fn test_println_macro_does_not_panic_when_we_go_beyond_vga_height() {
    for _ in 0..100 {
        println!("This should not panic!");
    }
}

#[test_case]
fn test_println_output_is_on_penultimate_line_and_uses_default_coloring() {
    let string_to_print = "Something that is less than 80 chars";
    execute_without_interrupts(|| {
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", string_to_print).expect("writeln failed");
        let height = writer.buffer_height();

        for (i, c) in string_to_print.chars().enumerate() {
            let screen_char = writer.char_at(height - 2, i);
            assert_eq!(screen_char, ScreenChar::with_default_coloring(c));
        }
    })
}
