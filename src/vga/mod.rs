//! The VGA text mode is a simple way to print text to the screen.

use core::fmt::Write;

use lazy_static::lazy_static;
use volatile::Volatile;

use crate::gen_iter;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const NEWLINE_BYTE: u8 = b'\n';
const ASCII_FALLBACK: u8 = 0xfe;
pub const VGA_SEGMENT_START: usize = 0xb8000;
lazy_static! {
    /// The default color used for printing. It is White
    pub static ref DEFAULT_COLOR_CODE: ColorCode = ColorCode::new(Color::White, Color::Black);
}
lazy_static! {
    /// The color uset to print errors. It is Red
    pub static ref ERROR_COLOR_CODE: ColorCode = ColorCode::new(Color::Red, Color::Black);
}
lazy_static! {
    /// The color uset to print errors. It is Red
    pub static ref WARN_COLOR_CODE: ColorCode = ColorCode::new(Color::Yellow, Color::Black);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
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

/// Full VGA color code.
///
/// Repesents the background and foregroud color That a character should be printed with.
///
/// The following table represents the color codes:
/// ```text
/// | Number | 	Color     |	Number + Bright Bit	| Bright Color |
/// | 0x0	 | Black      |	0x8	                | Dark Gray    |
/// | 0x1	 | Blue	      | 0x9	                | Light Blue   |
/// | 0x2	 | Green      |	0xa	                | Light Green
/// | 0x3	 | Cyan	      | 0xb	                | Light Cyan   |
/// | 0x4	 | Red	      | 0xc	                | Light Red    |
/// | 0x5	 | Magenta    |	0xd	                | Pink         |
/// | 0x6	 | Brown      |	0xe	                | Yellow       |
/// | 0x7	 | Light Gray | 0xf	                | White        |
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub fn new(foregroud: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | foregroud as u8)
    }
}

/// Ascii character with a color code.
///
/// The bit structure of the character is:
/// ```text
/// | Bit(s) |	Value            |
/// |   0-7  | 	ASCII code point |
/// |  8-11  |	Foreground color | Handled by the [`ColorCode`] struct
/// | 12-14  |	Background color | Handled by the [`ColorCode`] struct
/// |   15	 |  Blink            | (Not implemented)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    ascii_code_point: u8,
    color_code: ColorCode,
}

impl ScreenChar {
    #[cfg(test)]
    pub fn with_default_coloring(character: char) -> ScreenChar {
        ScreenChar {
            ascii_code_point: character as u8,
            color_code: *DEFAULT_COLOR_CODE,
        }
    }

    pub fn new(character: char, color: ColorCode) -> Self {
        ScreenChar {
            ascii_code_point: character as u8,
            color_code: color,
        }
    }
}

/// Representation of the VGA buffer. It is essentially a matrix of [`ScreenChar`]s. This matrix is
/// represented as is on the screen.
#[repr(transparent)]
pub struct Buffer {
    // The compiler doesn’t know that we really access VGA buffer memory (instead of normal RAM)
    // and knows nothing about the side effect that some characters appear on the screen. So it
    // might decide that these writes are unnecessary and can be omitted. To avoid this erroneous
    // optimization, we need to specify these writes as volatile. This tells the compiler that the
    // write has side effects and should not be optimized away.
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenLocation {
    row: usize,
    col: usize,
}

impl ScreenLocation {
    pub const fn new(row: usize, col: usize) -> Self {
        ScreenLocation { row, col }
    }
}

/// The Writer writes to the VGA buffer.
///
/// The writer will always write to the last line and shift lines up when a line is full (or on
/// \n). It keeps track of the current position in the last row. The current
/// foreground and background colors by making use of [`ColorCode`].
///
/// # Usage
/// ```
/// use vga_buffer::Writer;
///
/// let mut writer = Writer::new();
/// writer.write_string("Hello, world!");
/// ```
pub struct Writer {
    col_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Default for Writer {
    fn default() -> Self {
        Writer {
            col_position: 0,
            color_code: *DEFAULT_COLOR_CODE,
            buffer: unsafe { &mut *(VGA_SEGMENT_START as *mut Buffer) },
        }
    }
}

impl Writer {
    pub fn with_color_code(&mut self, color_code: ColorCode) -> &mut Self {
        self.color_code = color_code;
        self
    }

    /// Write a string slice to the VGA buffer.
    pub fn write_string(&mut self, string: &str) {
        for byte in string.as_bytes() {
            self.write_byte(self.color_code, *byte);
        }
    }

    pub fn char_at(&self, row: usize, col: usize) -> ScreenChar {
        self.buffer.chars[row][col].read()
    }

    pub fn buffer_height(&self) -> usize {
        BUFFER_HEIGHT
    }

    pub fn string_at(
        &self,
        start: ScreenLocation,
        end: ScreenLocation,
    ) -> impl Iterator<Item = ScreenChar> + '_ {
        gen_iter!(move {
            for row in start.row..=end.row {
                for col in start.col..=end.col {
                    yield self.char_at(row, col);
                }
            }
        })
    }

    pub fn clear_last_char(&mut self) {
        let blank = ScreenChar {
            color_code: self.color_code,
            ascii_code_point: b' ',
        };
        if self.col_position > 0 {
            self.buffer.chars[BUFFER_HEIGHT - 1][self.col_position - 1].write(blank);
            self.col_position -= 1;
        }
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
