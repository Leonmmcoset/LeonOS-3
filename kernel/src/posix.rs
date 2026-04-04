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

    pub fn read(&self, fd: usize, _buf: *mut u8, _len: usize) -> SysResult {
        match fd {
            STDIN_FILENO => 0,
            _ => -(EBADF as isize),
        }
    }
}
