use core::arch::asm;
use os_terminal::{DrawTarget, Terminal};

const COM1: u16 = 0x3F8;
static mut SERIAL_INIT: bool = false;

pub enum BootLevel {
    Ok,
    Warn,
    Fail,
    Info,
}

pub fn serial_write(s: &str) {
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

pub fn boot_line<D: DrawTarget>(terminal: &mut Terminal<D>, level: BootLevel, msg: &str) {
    match level {
        BootLevel::Ok => terminal.process(b"[  OK  ] "),
        BootLevel::Warn => terminal.process(b"[ WARN ] "),
        BootLevel::Fail => terminal.process(b"[ FAIL ] "),
        BootLevel::Info => terminal.process(b"[ .... ] "),
    }
    terminal.process(msg.as_bytes());
    terminal.process(b"\n");
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
