use core::fmt::{self, Write};

use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

use crate::x86_64::interrupts::execute_without_interrupts;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const NEWLINE_BYTE: u8 = b'\n';
const ASCII_FALLBACK: u8 = 0xfe;
const VGA_SEGMENT_START: usize = 0xb8000;
lazy_static! {
    pub static ref DEFAULT_COLOR_CODE: ColorCode = ColorCode::new(Color::White, Color::Black);
}
lazy_static! {
    pub static ref ERROR_COLOR_CODE: ColorCode = ColorCode::new(Color::Red, Color::Black);
}
lazy_static! {
    pub static ref SUCCESS_COLOR_CODE: ColorCode = ColorCode::new(Color::Green, Color::Black);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// | Number | 	Color     |	Number + Bright Bit	| Bright Color |
/// | 0x0	 | Black      |	0x8	                | Dark Gray    |
/// | 0x1	 | Blue	      | 0x9	                | Light Blue   |
/// | 0x2	 | Green      |	0xa	                | Light Green
/// | 0x3	 | Cyan	      | 0xb	                | Light Cyan   |
/// | 0x4	 | Red	      | 0xc	                | Light Red    |
/// | 0x5	 | Magenta    |	0xd	                | Pink         |
/// | 0x6	 | Brown      |	0xe	                | Yellow       |
/// | 0x7	 | Light Gray | 0xf	                | White        |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    fn new(foregroud: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | foregroud as u8)
    }
}

///
/// | Bit(s) |	Value            |
/// |   0-7  | 	ASCII code point |
/// |  8-11  |	Foreground color |
/// | 12-14  |	Background color |
/// |   15	 |  Blink            |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_code_point: u8,
    color_code: ColorCode,
}

impl ScreenChar {
    #[cfg(test)]
    fn with_default_coloring(character: char) -> ScreenChar {
        ScreenChar {
            ascii_code_point: character as u8,
            color_code: *DEFAULT_COLOR_CODE,
        }
    }
}

#[repr(transparent)]
struct Buffer {
    // The compiler doesn’t know that we really access VGA buffer memory (instead of normal RAM)
    // and knows nothing about the side effect that some characters appear on the screen. So it
    // might decide that these writes are unnecessary and can be omitted. To avoid this erroneous
    // optimization, we need to specify these writes as volatile. This tells the compiler that the
    // write has side effects and should not be optimized away.
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    col_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    fn with_color_code(&mut self, color_code: ColorCode) -> &mut Self {
        self.color_code = color_code;
        self
    }

    pub fn write_string(&mut self, string: &str) {
        for byte in string.as_bytes() {
            self.write_byte(self.color_code, *byte);
        }
    }

    #[cfg(test)]
    fn char_at(&self, row: usize, col: usize) -> ScreenChar {
        self.buffer.chars[row][col].read()
    }

    #[cfg(test)]
    pub fn buffer_height(&self) -> usize {
        BUFFER_HEIGHT
    }

    fn write_byte(&mut self, color_code: ColorCode, byte: u8) {
        match byte {
            // printable ascii characters
            0x20..=0x7e => self.write_byte_internal(color_code, byte),
            NEWLINE_BYTE => self.newline(),
            _ => self.write_byte_internal(color_code, ASCII_FALLBACK),
        }
    }

    fn newline(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                self.buffer.chars[row - 1][col].write(self.buffer.chars[row][col].read());
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.col_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            color_code: self.color_code,
            ascii_code_point: b' ',
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    fn write_byte_internal(&mut self, color_code: ColorCode, byte: u8) {
        if self.col_position >= BUFFER_WIDTH {
            self.newline()
        }

        let row = BUFFER_HEIGHT - 1;
        let col = self.col_position;
        let screen_char = ScreenChar {
            ascii_code_point: byte,
            color_code,
        };

        self.buffer.chars[row][col].write(screen_char);
        self.col_position += 1;
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        col_position: 0,
        color_code: *DEFAULT_COLOR_CODE,
        buffer: unsafe { &mut *(VGA_SEGMENT_START as *mut Buffer) },
    });
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => (print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => (
        $crate::vga_buffer::_print(
            *$crate::vga_buffer::DEFAULT_COLOR_CODE,
            format_args!($($arg)*)
        )
    );
}

#[macro_export]
macro_rules! errorln {
    () => (error!("\n"));
    ($($arg:tt)*) => (error!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => (
        $crate::vga_buffer::_print(
            *$crate::vga_buffer::ERROR_COLOR_CODE,
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
