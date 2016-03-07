
use core::ptr::Unique;
use spin::Mutex;

macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        $crate::vga::WRITER.lock().write_fmt(format_args!($($arg)*)).unwrap()
    });
}

macro_rules! print_locked {
    ($lock_guard:ident, $($arg:tt)*) => ({
        use core::fmt::Write;
        $lock_guard.write_fmt(format_args!($($arg)*)).unwrap()
    });
}

#[repr(u8)]
#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum Color {
    Black      = 0,
    Blue       = 1,
    Green      = 2,
    Cyan       = 3,
    Red        = 4,
    Magenta    = 5,
    Brown      = 6,
    LightGray  = 7,
    DarkGray   = 8,
    LightBlue  = 9,
    LightGreen = 10,
    LightCyan  = 11,
    LightRed   = 12,
    Pink       = 13,
    Yellow     = 14,
    White      = 15,
}

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(forground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (forground as u8))
    }
}

#[repr(C)]
#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
struct Character {
    ascii_character: u8,
    color: ColorCode,
}

const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;

struct Buffer {
    chars: [[Character; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: Unique<Buffer>,
}

pub static WRITER: Mutex<Writer> = Mutex::new(Writer {
    column_position: 0,
    color_code: ColorCode::new(Color::LightGreen, Color::Black),
    buffer: unsafe { Unique::new(0xb8000 as *mut _) },
});

impl Writer {
    pub fn set_color(&mut self, color: ColorCode) {
        self.color_code = color;
    }
    
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\t' => {
                self.write_byte(b' ');
                while self.column_position % 8 != 0 {
                    self.write_byte(b' ');
                }
            }
            byte => {
                if self.column_position == BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                self.buffer().chars[row][col] = Character {
                    ascii_character: byte,
                    color: self.color_code,
                };

                self.column_position += 1;
            }
        }
    }

    pub fn write_bytes(&mut self, b: &[u8]) {
        for &byte in b {
            self.write_byte(byte)
        }
    }

    fn buffer(&mut self) -> &mut Buffer {
        unsafe { self.buffer.get_mut() }
    }

    fn new_line(&mut self) {
        for i in 1..BUFFER_HEIGHT {
            let mut buffer = self.buffer();
            buffer.chars[i-1] = buffer.chars[i];
        }
        self.column_position = 0;
        self.clear_row(BUFFER_HEIGHT-1);
    }
    
    fn clear_row(&mut self, row: usize) {
        if row < BUFFER_HEIGHT {
            self.buffer().chars[row] = [Character {
                // Space
                ascii_character: 0x20,
                color: ColorCode::new(Color::White, Color::Black),
            }; BUFFER_WIDTH];
        }
    }

    fn clear_screen(&mut self) {
        for _ in 0..BUFFER_HEIGHT {
            println!("");
        }
    }
}

impl ::core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        self.write_bytes(s.as_bytes());
        Ok(())
    }
}
