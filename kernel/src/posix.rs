use core::hint::spin_loop;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub const STDIN_FILENO: usize = 0;
pub const STDOUT_FILENO: usize = 1;
pub const STDERR_FILENO: usize = 2;

pub const EBADF: isize = 9;
pub const EINVAL: isize = 22;

pub type SysResult = isize;

pub struct PosixLayer {
    pid: AtomicU32,
    brk: AtomicUsize,
}

static mut STDIN_READER: Option<fn() -> Option<u8>> = None;

pub fn set_stdin_reader(reader: fn() -> Option<u8>) {
    unsafe {
        STDIN_READER = Some(reader);
    }
}

pub fn read_stdin_byte_blocking() -> Option<u8> {
    let reader = unsafe { STDIN_READER };
    let Some(reader_fn) = reader else {
        return None;
    };

    loop {
        if let Some(ch) = reader_fn() {
            return Some(ch);
        }
        spin_loop();
    }
}

impl PosixLayer {
    pub const fn new() -> Self {
        Self {
            pid: AtomicU32::new(3),
            brk: AtomicUsize::new(0x4000_0000),
        }
    }

    pub fn getpid(&self) -> u32 {
        self.pid.load(Ordering::Relaxed)
    }

    pub fn brk_get(&self) -> usize {
        self.brk.load(Ordering::Relaxed)
    }

    pub fn brk_set(&self, new_brk: usize) -> usize {
        if new_brk != 0 {
            self.brk.store(new_brk, Ordering::Relaxed);
        }
        self.brk_get()
    }

    pub fn write(&self, fd: usize, _buf: *const u8, len: usize) -> SysResult {
        match fd {
            STDOUT_FILENO | STDERR_FILENO => len as isize,
            _ => -(EBADF as isize),
        }
    }

    pub fn read(&self, fd: usize, buf: *mut u8, len: usize) -> SysResult {
        match fd {
            STDIN_FILENO => {
                if buf.is_null() || len == 0 {
                    return 0;
                }

                let reader = unsafe { STDIN_READER };
                let Some(reader_fn) = reader else {
                    return 0;
                };

                let mut n = 0usize;
                while n < len {
                    if let Some(ch) = reader_fn() {
                        unsafe {
                            *buf.add(n) = ch;
                        }
                        n += 1;
                        if ch == b'\n' || ch == b'\r' {
                            break;
                        }
                    } else {
                        spin_loop();
                    }
                }

                n as isize
            }
            _ => -(EBADF as isize),
        }
    }
}


