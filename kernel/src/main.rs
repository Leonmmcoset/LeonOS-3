#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

mod allocator;
mod shell;

use alloc::boxed::Box;
use bootloader_api::config::{BootloaderConfig, Mapping};
use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use bootloader_api::{entry_point, BootInfo};
use core::alloc::Layout;
use core::arch::asm;
use core::hint::spin_loop;
use core::panic::PanicInfo;
use os_terminal::font::BitmapFont;
use os_terminal::{DrawTarget, Rgb, Terminal};

const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

struct Display {
    info: FrameBufferInfo,
    buffer: &'static mut [u8],
}

impl DrawTarget for Display {
    fn size(&self) -> (usize, usize) {
        (self.info.width, self.info.height)
    }

    #[inline(always)]
    fn draw_pixel(&mut self, x: usize, y: usize, color: Rgb) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }

        let pixel_index = y * self.info.stride + x;
        let byte_index = pixel_index * self.info.bytes_per_pixel;
        let pixel = &mut self.buffer[byte_index..byte_index + self.info.bytes_per_pixel];

        match self.info.pixel_format {
            PixelFormat::Rgb => {
                pixel[0] = color.0;
                pixel[1] = color.1;
                pixel[2] = color.2;
            }
            PixelFormat::Bgr => {
                pixel[0] = color.2;
                pixel[1] = color.1;
                pixel[2] = color.0;
            }
            PixelFormat::U8 => {
                let gray = ((color.0 as u16 + color.1 as u16 + color.2 as u16) / 3) as u8;
                pixel[0] = gray;
            }
            _ => {
                pixel[0] = color.0;
                if self.info.bytes_per_pixel > 1 {
                    pixel[1] = color.1;
                }
                if self.info.bytes_per_pixel > 2 {
                    pixel[2] = color.2;
                }
            }
        }
    }
}

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    allocator::init_heap();
    serial_write("[LeonOS3] kernel entered\n");

    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        serial_write("[LeonOS3] framebuffer present\n");

        let info = framebuffer.info();
        let buffer = framebuffer.buffer_mut();
        buffer.fill(0);

        let display = Display { info, buffer };
        let mut terminal = Terminal::new(display, Box::new(BitmapFont));
        terminal.set_crnl_mapping(true);
        terminal.set_auto_flush(true);

        terminal.process(b"\x1b[32mLeonOS 3 booting...\x1b[0m\n");
        terminal.process(b"Simple shell ready. Type 'help'.\n\n");
        serial_write("[LeonOS3] shell ready\n");

        shell::run_shell(&mut terminal, keyboard_read_ascii_nonblocking)
    } else {
        serial_write("[LeonOS3] no framebuffer, using VGA fallback\n");
        vga_fallback_print("LeonOS 3 booting...\nHelloWorld (no framebuffer)");
        loop {
            x86_64::instructions::hlt();
        }
    }
}

const COM1: u16 = 0x3F8;
const PS2_DATA: u16 = 0x60;
const PS2_STATUS: u16 = 0x64;

static mut SERIAL_INIT: bool = false;
static mut SHIFT_DOWN: bool = false;

fn serial_write(s: &str) {
    unsafe {
        if !SERIAL_INIT {
            serial_init();
            SERIAL_INIT = true;
        }

        for b in s.bytes() {
            if b == b'\n' {
                serial_write_byte(b'\r');
            }
            serial_write_byte(b);
        }
    }
}

fn keyboard_read_ascii_nonblocking() -> Option<u8> {
    unsafe {
        if (inb(PS2_STATUS) & 0x01) == 0 {
            return None;
        }

        let sc = inb(PS2_DATA);

        match sc {
            0x2A | 0x36 => {
                SHIFT_DOWN = true;
                return None;
            }
            0xAA | 0xB6 => {
                SHIFT_DOWN = false;
                return None;
            }
            _ => {}
        }

        if (sc & 0x80) != 0 {
            return None;
        }

        scancode_to_ascii(sc, SHIFT_DOWN)
    }
}

fn scancode_to_ascii(sc: u8, shift: bool) -> Option<u8> {
    let ch = match sc {
        0x1C => b'\n',
        0x0E => 8,
        0x39 => b' ',
        0x02 => if shift { b'!' } else { b'1' },
        0x03 => if shift { b'@' } else { b'2' },
        0x04 => if shift { b'#' } else { b'3' },
        0x05 => if shift { b'$' } else { b'4' },
        0x06 => if shift { b'%' } else { b'5' },
        0x07 => if shift { b'^' } else { b'6' },
        0x08 => if shift { b'&' } else { b'7' },
        0x09 => if shift { b'*' } else { b'8' },
        0x0A => if shift { b'(' } else { b'9' },
        0x0B => if shift { b')' } else { b'0' },
        0x0C => if shift { b'_' } else { b'-' },
        0x0D => if shift { b'+' } else { b'=' },
        0x10 => if shift { b'Q' } else { b'q' },
        0x11 => if shift { b'W' } else { b'w' },
        0x12 => if shift { b'E' } else { b'e' },
        0x13 => if shift { b'R' } else { b'r' },
        0x14 => if shift { b'T' } else { b't' },
        0x15 => if shift { b'Y' } else { b'y' },
        0x16 => if shift { b'U' } else { b'u' },
        0x17 => if shift { b'I' } else { b'i' },
        0x18 => if shift { b'O' } else { b'o' },
        0x19 => if shift { b'P' } else { b'p' },
        0x1E => if shift { b'A' } else { b'a' },
        0x1F => if shift { b'S' } else { b's' },
        0x20 => if shift { b'D' } else { b'd' },
        0x21 => if shift { b'F' } else { b'f' },
        0x22 => if shift { b'G' } else { b'g' },
        0x23 => if shift { b'H' } else { b'h' },
        0x24 => if shift { b'J' } else { b'j' },
        0x25 => if shift { b'K' } else { b'k' },
        0x26 => if shift { b'L' } else { b'l' },
        0x2C => if shift { b'Z' } else { b'z' },
        0x2D => if shift { b'X' } else { b'x' },
        0x2E => if shift { b'C' } else { b'c' },
        0x2F => if shift { b'V' } else { b'v' },
        0x30 => if shift { b'B' } else { b'b' },
        0x31 => if shift { b'N' } else { b'n' },
        0x32 => if shift { b'M' } else { b'm' },
        _ => return None,
    };

    Some(ch)
}

unsafe fn serial_init() {
    outb(COM1 + 1, 0x00);
    outb(COM1 + 3, 0x80);
    outb(COM1 + 0, 0x03);
    outb(COM1 + 1, 0x00);
    outb(COM1 + 3, 0x03);
    outb(COM1 + 2, 0xC7);
    outb(COM1 + 4, 0x0B);
}

unsafe fn serial_write_byte(byte: u8) {
    while (inb(COM1 + 5) & 0x20) == 0 {}
    outb(COM1, byte);
}

unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value);
}

unsafe fn inb(port: u16) -> u8 {
    let mut value: u8;
    asm!("in al, dx", in("dx") port, out("al") value);
    value
}

fn vga_fallback_print(s: &str) {
    let mut offset = 0usize;
    for &b in s.as_bytes() {
        if b == b'\n' {
            let row = offset / 80;
            offset = (row + 1) * 80;
            continue;
        }
        if offset >= 80 * 25 {
            break;
        }
        let cell = 0xb8000 as *mut u8;
        unsafe {
            core::ptr::write_volatile(cell.add(offset * 2), b);
            core::ptr::write_volatile(cell.add(offset * 2 + 1), 0x0f);
        }
        offset += 1;
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_write("[LeonOS3] panic\n");
    loop {
        spin_loop();
    }
}

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    serial_write("[LeonOS3] alloc error\n");
    loop {
        spin_loop();
    }
}
