use crate::posix::{PosixLayer, EINVAL};
use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_CLOSE: usize = 3;
pub const SYS_STAT: usize = 4;
pub const SYS_FSTAT: usize = 5;
pub const SYS_LSTAT: usize = 6;
pub const SYS_POLL: usize = 7;
pub const SYS_LSEEK: usize = 8;
pub const SYS_MMAP: usize = 9;
pub const SYS_MPROTECT: usize = 10;
pub const SYS_MUNMAP: usize = 11;
pub const SYS_BRK: usize = 12;
pub const SYS_RT_SIGACTION: usize = 13;
pub const SYS_RT_SIGPROCMASK: usize = 14;
pub const SYS_IOCTL: usize = 16;
pub const SYS_PREAD64: usize = 17;
pub const SYS_PWRITE64: usize = 18;
pub const SYS_READV: usize = 19;
pub const SYS_WRITEV: usize = 20;
pub const SYS_ACCESS: usize = 21;
pub const SYS_PIPE: usize = 22;
pub const SYS_SELECT: usize = 23;
pub const SYS_SCHED_YIELD: usize = 24;
pub const SYS_MREMAP: usize = 25;
pub const SYS_MSYNC: usize = 26;
pub const SYS_MINCORE: usize = 27;
pub const SYS_MADVISE: usize = 28;
pub const SYS_DUP: usize = 32;
pub const SYS_DUP2: usize = 33;
pub const SYS_NANOSLEEP: usize = 35;
pub const SYS_GETPID: usize = 39;
pub const SYS_SOCKET: usize = 41;
pub const SYS_CONNECT: usize = 42;
pub const SYS_ACCEPT: usize = 43;
pub const SYS_SENDTO: usize = 44;
pub const SYS_RECVFROM: usize = 45;
pub const SYS_SHUTDOWN: usize = 48;
pub const SYS_BIND: usize = 49;
pub const SYS_LISTEN: usize = 50;
pub const SYS_CLONE: usize = 56;
pub const SYS_FORK: usize = 57;
pub const SYS_VFORK: usize = 58;
pub const SYS_EXECVE: usize = 59;
pub const SYS_EXIT: usize = 60;
pub const SYS_WAIT4: usize = 61;
pub const SYS_UNAME: usize = 63;
pub const SYS_FCNTL: usize = 72;
pub const SYS_FSYNC: usize = 74;
pub const SYS_FDATASYNC: usize = 75;
pub const SYS_TRUNCATE: usize = 76;
pub const SYS_FTRUNCATE: usize = 77;
pub const SYS_GETCWD: usize = 79;
pub const SYS_CHDIR: usize = 80;
pub const SYS_RENAME: usize = 82;
pub const SYS_MKDIR: usize = 83;
pub const SYS_RMDIR: usize = 84;
pub const SYS_CREAT: usize = 85;
pub const SYS_LINK: usize = 86;
pub const SYS_UNLINK: usize = 87;
pub const SYS_READLINK: usize = 89;
pub const SYS_CHMOD: usize = 90;
pub const SYS_CHOWN: usize = 92;
pub const SYS_GETTIMEOFDAY: usize = 96;
pub const SYS_GETUID: usize = 102;
pub const SYS_GETGID: usize = 104;
pub const SYS_GETEUID: usize = 107;
pub const SYS_GETEGID: usize = 108;
pub const SYS_GETPPID: usize = 110;
pub const SYS_GETPGRP: usize = 111;
pub const SYS_SETSID: usize = 112;
pub const SYS_SIGALTSTACK: usize = 131;
pub const SYS_ARCH_PRCTL: usize = 158;
pub const SYS_GETTID: usize = 186;
pub const SYS_FUTEX: usize = 202;
pub const SYS_CLOCK_GETTIME: usize = 228;
pub const SYS_EXIT_GROUP: usize = 231;
pub const SYS_OPENAT: usize = 257;
pub const SYS_MKDIRAT: usize = 258;
pub const SYS_MKNODAT: usize = 259;
pub const SYS_NEWFSTATAT: usize = 262;
pub const SYS_UNLINKAT: usize = 263;
pub const SYS_RENAMEAT: usize = 264;
pub const SYS_READLINKAT: usize = 267;
pub const SYS_FACCESSAT: usize = 269;
pub const SYS_PSELECT6: usize = 270;
pub const SYS_PPOLL: usize = 271;
pub const SYS_SET_ROBUST_LIST: usize = 273;
pub const SYS_PRLIMIT64: usize = 302;
pub const SYS_GETRANDOM: usize = 318;
pub const SYS_MEMFD_CREATE: usize = 319;
pub const SYS_RSEQ: usize = 334;
pub const SYS_STATX: usize = 332;
pub const SYS_GETDENTS64: usize = 217;
pub const SYS_SET_TID_ADDRESS: usize = 218;

const ENOSYS: isize = 38;
const ENOENT: isize = 2;
const EBADF: isize = 9;
const ENOTTY: isize = 25;

static POSIX: PosixLayer = PosixLayer::new();
static LAST_EXIT: AtomicI32 = AtomicI32::new(0);
static MMAP_BASE: AtomicU64 = AtomicU64::new(0x7000_0000);
static ARCH_FS: AtomicU64 = AtomicU64::new(0);
static ARCH_GS: AtomicU64 = AtomicU64::new(0);

pub fn dispatch(
    nr: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    _arg4: usize,
    _arg5: usize,
) -> isize {
    match nr {
        SYS_READ => POSIX.read(arg0, arg1 as *mut u8, arg2),
        SYS_WRITE => POSIX.write(arg0, arg1 as *const u8, arg2),
        SYS_BRK => POSIX.brk_set(arg0) as isize,
        SYS_GETPID => POSIX.getpid() as isize,
        SYS_GETTID => POSIX.getpid() as isize,
        SYS_SET_TID_ADDRESS => POSIX.getpid() as isize,
        SYS_EXIT | SYS_EXIT_GROUP => {
            LAST_EXIT.store(arg0 as i32, Ordering::Relaxed);
            0
        }

        SYS_CLOSE => {
            if arg0 <= 2 { 0 } else { -(EBADF as isize) }
        }

        SYS_LSEEK => {
            let _fd = arg0;
            let off = arg1 as isize;
            let _whence = arg2;
            off
        }

        SYS_IOCTL => -(ENOTTY as isize),

        SYS_MMAP => {
            let len = (arg1 as u64).max(0x1000);
            let align_len = (len + 0xFFF) & !0xFFF;
            MMAP_BASE.fetch_add(align_len, Ordering::Relaxed) as isize
        }
        SYS_MPROTECT | SYS_MUNMAP => 0,

        SYS_RT_SIGACTION | SYS_RT_SIGPROCMASK | SYS_SIGALTSTACK => 0,

        SYS_CLOCK_GETTIME | SYS_GETTIMEOFDAY | SYS_NANOSLEEP => 0,

        SYS_UNAME => 0,

        SYS_GETUID | SYS_GETEUID | SYS_GETGID | SYS_GETEGID => 0,
        SYS_GETPPID => 1,
        SYS_GETPGRP => 1,
        SYS_SETSID => 1,

        SYS_ARCH_PRCTL => {
            // x86_64: ARCH_SET_GS=0x1001, ARCH_SET_FS=0x1002, ARCH_GET_FS=0x1003, ARCH_GET_GS=0x1004
            match arg0 {
                0x1001 => {
                    ARCH_GS.store(arg1 as u64, Ordering::Relaxed);
                    0
                }
                0x1002 => {
                    ARCH_FS.store(arg1 as u64, Ordering::Relaxed);
                    0
                }
                0x1003 | 0x1004 => 0,
                _ => -(EINVAL as isize),
            }
        }

        SYS_SET_ROBUST_LIST | SYS_RSEQ | SYS_PRLIMIT64 | SYS_FUTEX => 0,
        SYS_GETRANDOM => arg1 as isize,

        SYS_OPENAT => {
            let _dirfd = arg0;
            let path_ptr = arg1;
            let _flags = arg2;
            let _mode = arg3;
            if path_ptr == 0 {
                -(EINVAL as isize)
            } else {
                -(ENOENT as isize)
            }
        }

        SYS_NEWFSTATAT | SYS_STAT | SYS_FSTAT | SYS_LSTAT | SYS_STATX => 0,
        SYS_GETDENTS64 => 0,
        SYS_READLINKAT | SYS_READLINK => -(ENOENT as isize),
        SYS_FACCESSAT | SYS_ACCESS => 0,

        SYS_DUP | SYS_DUP2 | SYS_FCNTL | SYS_FSYNC | SYS_FDATASYNC | SYS_TRUNCATE | SYS_FTRUNCATE => 0,
        SYS_GETCWD | SYS_CHDIR | SYS_RENAME | SYS_MKDIR | SYS_RMDIR | SYS_CREAT | SYS_LINK | SYS_UNLINK
        | SYS_CHMOD | SYS_CHOWN | SYS_MKDIRAT | SYS_MKNODAT | SYS_UNLINKAT | SYS_RENAMEAT => 0,

        SYS_PREAD64 | SYS_PWRITE64 | SYS_READV | SYS_WRITEV | SYS_PSELECT6 | SYS_PPOLL | SYS_POLL | SYS_SELECT => 0,

        SYS_SOCKET | SYS_CONNECT | SYS_ACCEPT | SYS_SENDTO | SYS_RECVFROM | SYS_SHUTDOWN | SYS_BIND | SYS_LISTEN => -(ENOSYS as isize),
        SYS_CLONE | SYS_FORK | SYS_VFORK | SYS_EXECVE | SYS_WAIT4 | SYS_PIPE | SYS_MREMAP | SYS_MSYNC | SYS_MINCORE | SYS_MADVISE
        | SYS_MEMFD_CREATE => -(ENOSYS as isize),

        _ => -(ENOSYS as isize),
    }
}

pub fn last_exit_code() -> i32 {
    LAST_EXIT.load(Ordering::Relaxed)
}

pub fn posix_snapshot() -> (u32, usize) {
    (POSIX.getpid(), POSIX.brk_get())
}

pub fn supported_syscalls_hint() -> &'static str {
    "read, write, close, brk, mmap, mprotect, munmap, rt_sigaction, rt_sigprocmask, ioctl, getpid/gettid, exit/exit_group, arch_prctl, futex, clock_gettime, uname, uid/gid, openat(newfstatat/getdents64 stubs), getrandom, prlimit64, set_robust_list"
}
