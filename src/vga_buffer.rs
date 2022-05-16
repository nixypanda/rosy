use core::fmt::{self, Write};

use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const NEWLINE_BYTE: u8 = b'\n';
const ASCII_FALLBACK: u8 = 0xfe;
const VGA_SEGMENT_START: usize = 0xb8000;

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
struct ColorCode(u8);

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
    pub fn write_string(&mut self, string: &str) {
        for byte in string.as_bytes() {
            self.write_byte(*byte);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            // printable ascii characters
            0x20..=0x7e => self.write_byte_internal(byte),
            NEWLINE_BYTE => self.newline(),
            _ => self.write_byte_internal(ASCII_FALLBACK),
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

    fn write_byte_internal(&mut self, byte: u8) {
        if self.col_position >= BUFFER_WIDTH {
            self.newline()
        }

        let row = BUFFER_HEIGHT - 1;
        let col = self.col_position;
        let color_code = self.color_code;
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
        color_code: ColorCode::new(Color::Yellow, Color::Black),
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
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    WRITER.lock().write_fmt(args).unwrap();
}
