use core::fmt::{self, Write};
use spin::Mutex;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const VGA_BUFFER_ADDR: usize = 0xb8000;

#[allow(dead_code)]
#[repr(u8)]
enum Color {
    Black = 0,
    LightGray = 7,
    Yellow = 14,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    const fn new(foreground: Color, background: Color) -> Self {
        Self((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

pub struct OSTerminal {
    row: usize,
    col: usize,
    color_code: ColorCode,
}

impl OSTerminal {
    const fn new() -> Self {
        Self {
            row: 0,
            col: 0,
            color_code: ColorCode::new(Color::Yellow, Color::Black),
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.col >= BUFFER_WIDTH {
                    self.new_line();
                }
                self.write_at(self.row, self.col, byte, self.color_code);
                self.col += 1;
            }
        }
    }

    fn write_at(&self, row: usize, col: usize, byte: u8, color: ColorCode) {
        let index = row * BUFFER_WIDTH + col;
        let ptr = (VGA_BUFFER_ADDR as *mut ScreenChar).wrapping_add(index);
        let ch = ScreenChar {
            ascii_character: byte,
            color_code: color,
        };
        // Safety: VGA text buffer is memory-mapped at 0xb8000 in x86_64 text mode.
        unsafe { core::ptr::write_volatile(ptr, ch) };
    }

    fn new_line(&mut self) {
        if self.row + 1 >= BUFFER_HEIGHT {
            self.scroll_up();
        } else {
            self.row += 1;
        }
        self.col = 0;
    }

    fn scroll_up(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let from_index = row * BUFFER_WIDTH + col;
                let to_index = (row - 1) * BUFFER_WIDTH + col;
                let from_ptr = (VGA_BUFFER_ADDR as *const ScreenChar).wrapping_add(from_index);
                let to_ptr = (VGA_BUFFER_ADDR as *mut ScreenChar).wrapping_add(to_index);
                // Safety: both pointers target valid VGA text buffer cells.
                unsafe {
                    let ch = core::ptr::read_volatile(from_ptr);
                    core::ptr::write_volatile(to_ptr, ch);
                }
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.row = BUFFER_HEIGHT - 1;
        self.col = 0;
    }

    fn clear_row(&mut self, row: usize) {
        for col in 0..BUFFER_WIDTH {
            self.write_at(row, col, b' ', self.color_code);
        }
    }

    fn clear_screen_inner(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);
        }
        self.row = 0;
        self.col = 0;
    }

    pub fn clear_screen() {
        WRITER.lock().clear_screen_inner();
    }

    pub fn print(s: &str) {
        let _ = WRITER.lock().write_str(s);
    }

    pub fn println(s: &str) {
        let mut w = WRITER.lock();
        let _ = w.write_str(s);
        let _ = w.write_str("\n");
    }

    pub fn print_fmt(args: fmt::Arguments<'_>) {
        let _ = WRITER.lock().write_fmt(args);
    }
}

impl Write for OSTerminal {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
        Ok(())
    }
}

static WRITER: Mutex<OSTerminal> = Mutex::new(OSTerminal::new());
