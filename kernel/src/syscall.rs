use crate::fs;
use crate::posix::{self, PosixLayer, EINVAL};
use core::cmp::min;
use core::mem::size_of;
use core::ptr;
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
pub const SYS_GETDENTS64: usize = 217;
pub const SYS_SET_TID_ADDRESS: usize = 218;
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
pub const SYS_STATX: usize = 332;
pub const SYS_RSEQ: usize = 334;

const ENOENT: isize = 2;
const EBADF: isize = 9;
const EFAULT: isize = 14;
const EBUSY: isize = 16;
const ENOTDIR: isize = 20;
const EISDIR: isize = 21;
const ENOSYS: isize = 38;
const ERANGE: isize = 34;
const ENOTTY: isize = 25;

const AT_FDCWD: isize = -100;
const AT_EMPTY_PATH: usize = 0x1000;

const O_DIRECTORY: usize = 0o200000;

const SEEK_SET: usize = 0;
const SEEK_CUR: usize = 1;
const SEEK_END: usize = 2;

const DT_DIR: u8 = 4;
const DT_REG: u8 = 8;

const MAX_FDS: usize = 64;
const FD_BASE: usize = 3;
const CSTR_MAX: usize = 256;

static POSIX: PosixLayer = PosixLayer::new();
static LAST_EXIT: AtomicI32 = AtomicI32::new(0);
static MMAP_BASE: AtomicU64 = AtomicU64::new(0x7000_0000);
static ARCH_FS: AtomicU64 = AtomicU64::new(0);
static ARCH_GS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum FdEntry {
    Free,
    RootDir { pos: usize },
    File {
        data_ptr: *const u8,
        len: usize,
        pos: usize,
        mode: u16,
    },
}

#[repr(C)]
struct LinuxStat {
    st_dev: u64,
    st_ino: u64,
    st_nlink: u64,
    st_mode: u32,
    st_uid: u32,
    st_gid: u32,
    __pad0: u32,
    st_rdev: u64,
    st_size: i64,
    st_blksize: i64,
    st_blocks: i64,
    st_atime: i64,
    st_atime_nsec: i64,
    st_mtime: i64,
    st_mtime_nsec: i64,
    st_ctime: i64,
    st_ctime_nsec: i64,
    __unused: [i64; 3],
}

static mut ROOTFS: Option<&'static [u8]> = None;
static mut CWD_BUF: [u8; 64] = [0; 64];
static mut CWD_LEN: usize = 1;
static mut FD_TABLE: [FdEntry; MAX_FDS] = [FdEntry::Free; MAX_FDS];

pub fn set_rootfs(image: &'static [u8]) {
    unsafe {
        ROOTFS = Some(image);
        FD_TABLE = [FdEntry::Free; MAX_FDS];
        CWD_BUF[0] = b'/';
        CWD_LEN = 1;
    }
}

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
        SYS_READ => {
            if arg0 <= 2 {
                POSIX.read(arg0, arg1 as *mut u8, arg2)
            } else {
                sys_read(arg0, arg1 as *mut u8, arg2)
            }
        }
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
            if arg0 <= 2 {
                0
            } else {
                sys_close(arg0)
            }
        }

        SYS_LSEEK => {
            if arg0 <= 2 {
                arg1 as isize
            } else {
                sys_lseek(arg0, arg1 as isize, arg2)
            }
        }

        SYS_IOCTL => -(ENOTTY as isize),

        SYS_MMAP => {
            let len = (arg1 as u64).max(0x1000);
            let align_len = (len + 0xFFF) & !0xFFF;
            MMAP_BASE.fetch_add(align_len, Ordering::Relaxed) as isize
        }
        SYS_MPROTECT | SYS_MUNMAP => 0,

        SYS_RT_SIGACTION | SYS_RT_SIGPROCMASK | SYS_SIGALTSTACK => 0,

        SYS_CLOCK_GETTIME => sys_clock_gettime(arg0, arg1 as *mut u8),
        SYS_GETTIMEOFDAY => sys_gettimeofday(arg0 as *mut u8, arg1 as *mut u8),
        SYS_NANOSLEEP => 0,

        SYS_UNAME => sys_uname(arg0 as *mut u8),

        SYS_GETUID | SYS_GETEUID | SYS_GETGID | SYS_GETEGID => 0,
        SYS_GETPPID => 1,
        SYS_GETPGRP => 1,
        SYS_SETSID => 1,

        SYS_ARCH_PRCTL => match arg0 {
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
        },

        SYS_SET_ROBUST_LIST | SYS_RSEQ | SYS_PRLIMIT64 | SYS_FUTEX => 0,
        SYS_GETRANDOM => sys_getrandom(arg0 as *mut u8, arg1),

        SYS_OPENAT => sys_openat(arg0 as isize, arg1 as *const u8, arg2, arg3),

        SYS_NEWFSTATAT => sys_newfstatat(arg0 as isize, arg1 as *const u8, arg2 as *mut u8, arg3),
        SYS_STAT | SYS_LSTAT => sys_stat_path(arg0 as *const u8, arg1 as *mut u8),
        SYS_FSTAT => sys_fstat(arg0, arg1 as *mut u8),
        SYS_STATX => 0,

        SYS_GETDENTS64 => sys_getdents64(arg0, arg1 as *mut u8, arg2),

        SYS_READLINKAT => sys_readlinkat(arg0 as isize, arg1 as *const u8, arg2 as *mut u8, arg3),
        SYS_READLINK => sys_readlink(arg0 as *const u8, arg1 as *mut u8, arg2),
        SYS_FACCESSAT => sys_faccessat(arg0 as isize, arg1 as *const u8),
        SYS_ACCESS => sys_access(arg0 as *const u8),

        SYS_DUP => sys_dup(arg0),
        SYS_DUP2 => sys_dup2(arg0, arg1),
        SYS_FCNTL | SYS_FSYNC | SYS_FDATASYNC | SYS_TRUNCATE | SYS_FTRUNCATE => 0,
        SYS_GETCWD => sys_getcwd(arg0 as *mut u8, arg1),
        SYS_CHDIR => sys_chdir(arg0 as *const u8),
        SYS_RENAME | SYS_MKDIR | SYS_RMDIR | SYS_CREAT | SYS_LINK | SYS_UNLINK
        | SYS_CHMOD | SYS_CHOWN | SYS_MKDIRAT | SYS_MKNODAT | SYS_UNLINKAT | SYS_RENAMEAT => 0,

        SYS_PREAD64 | SYS_PWRITE64 | SYS_READV | SYS_WRITEV | SYS_PSELECT6 | SYS_PPOLL | SYS_POLL | SYS_SELECT => 0,

        SYS_SOCKET | SYS_CONNECT | SYS_ACCEPT | SYS_SENDTO | SYS_RECVFROM | SYS_SHUTDOWN | SYS_BIND | SYS_LISTEN => {
            -(ENOSYS as isize)
        }
        SYS_CLONE
        | SYS_FORK
        | SYS_VFORK
        | SYS_EXECVE
        | SYS_WAIT4
        | SYS_PIPE
        | SYS_MREMAP
        | SYS_MSYNC
        | SYS_MINCORE
        | SYS_MADVISE
        | SYS_MEMFD_CREATE => -(ENOSYS as isize),

        _ => -(ENOSYS as isize),
    }
}

fn rootfs() -> Result<&'static [u8], isize> {
    unsafe { ROOTFS.ok_or(-(ENOENT as isize)) }
}

fn alloc_fd(entry: FdEntry) -> isize {
    unsafe {
        for i in FD_BASE..MAX_FDS {
            if matches!(FD_TABLE[i], FdEntry::Free) {
                FD_TABLE[i] = entry;
                return i as isize;
            }
        }
    }
    -(EBUSY as isize)
}

fn sys_openat(dirfd: isize, path_ptr: *const u8, flags: usize, _mode: usize) -> isize {
    if path_ptr.is_null() {
        return -(EFAULT as isize);
    }

    if !is_dirfd_ok(dirfd) {
        return -(EBADF as isize);
    }

    let path = match read_cstr(path_ptr) {
        Some(p) => p,
        None => return -(EFAULT as isize),
    };

    if path.is_empty() || path == "." || path == "/" {
        return alloc_fd(FdEntry::RootDir { pos: 0 });
    }

    if path == ".." {
        return alloc_fd(FdEntry::RootDir { pos: 0 });
    }

    let Some(name) = normalize_path(path) else {
        return -(ENOENT as isize);
    };

    if name.is_empty() {
        return alloc_fd(FdEntry::RootDir { pos: 0 });
    }

    let fs_image = match rootfs() {
        Ok(r) => r,
        Err(e) => return e,
    };

    match fs::open(fs_image, name) {
        Ok(rec) => {
            if (flags & O_DIRECTORY) != 0 {
                return -(ENOTDIR as isize);
            }
            alloc_fd(FdEntry::File {
                data_ptr: rec.data.as_ptr(),
                len: rec.data.len(),
                pos: 0,
                mode: rec.mode,
            })
        }
        Err(_) => -(ENOENT as isize),
    }
}

fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    if buf.is_null() {
        return -(EFAULT as isize);
    }

    unsafe {
        if fd >= MAX_FDS {
            return -(EBADF as isize);
        }

        match &mut FD_TABLE[fd] {
            FdEntry::Free => -(EBADF as isize),
            FdEntry::RootDir { .. } => -(EISDIR as isize),
            FdEntry::File { data_ptr, len: total, pos, .. } => {
                if *pos >= *total {
                    return 0;
                }
                let n = min(len, *total - *pos);
                let src = data_ptr.add(*pos);
                ptr::copy_nonoverlapping(src, buf, n);
                *pos += n;
                n as isize
            }
        }
    }
}

fn sys_close(fd: usize) -> isize {
    unsafe {
        if fd >= MAX_FDS {
            return -(EBADF as isize);
        }
        match FD_TABLE[fd] {
            FdEntry::Free => -(EBADF as isize),
            _ => {
                FD_TABLE[fd] = FdEntry::Free;
                0
            }
        }
    }
}

fn sys_lseek(fd: usize, off: isize, whence: usize) -> isize {
    unsafe {
        if fd >= MAX_FDS {
            return -(EBADF as isize);
        }

        match &mut FD_TABLE[fd] {
            FdEntry::Free => -(EBADF as isize),
            FdEntry::RootDir { pos } => {
                let base = match whence {
                    SEEK_SET => 0isize,
                    SEEK_CUR => *pos as isize,
                    SEEK_END => root_entries_count() as isize,
                    _ => return -(EINVAL as isize),
                };
                let new_pos = base.saturating_add(off);
                if new_pos < 0 {
                    return -(EINVAL as isize);
                }
                *pos = new_pos as usize;
                *pos as isize
            }
            FdEntry::File { len, pos, .. } => {
                let base = match whence {
                    SEEK_SET => 0isize,
                    SEEK_CUR => *pos as isize,
                    SEEK_END => *len as isize,
                    _ => return -(EINVAL as isize),
                };
                let new_pos = base.saturating_add(off);
                if new_pos < 0 {
                    return -(EINVAL as isize);
                }
                *pos = new_pos as usize;
                *pos as isize
            }
        }
    }
}

fn sys_stat_path(path_ptr: *const u8, stat_ptr: *mut u8) -> isize {
    if path_ptr.is_null() || stat_ptr.is_null() {
        return -(EFAULT as isize);
    }

    let path = match read_cstr(path_ptr) {
        Some(p) => p,
        None => return -(EFAULT as isize),
    };

    if path.is_empty() || path == "." || path == "/" {
        return write_stat(stat_ptr, true, 0, 0o040755, 1);
    }

    let Some(name) = normalize_path(path) else {
        return -(ENOENT as isize);
    };

    let fs_image = match rootfs() {
        Ok(r) => r,
        Err(e) => return e,
    };

    match fs::open(fs_image, name) {
        Ok(rec) => write_stat(stat_ptr, false, rec.data.len(), rec.mode, 2),
        Err(_) => -(ENOENT as isize),
    }
}

fn sys_fstat(fd: usize, stat_ptr: *mut u8) -> isize {
    if stat_ptr.is_null() {
        return -(EFAULT as isize);
    }

    if fd <= 2 {
        return write_stat(stat_ptr, false, 0, 0o020666, 1);
    }

    unsafe {
        if fd >= MAX_FDS {
            return -(EBADF as isize);
        }
        match FD_TABLE[fd] {
            FdEntry::Free => -(EBADF as isize),
            FdEntry::RootDir { .. } => write_stat(stat_ptr, true, 0, 0o040755, 1),
            FdEntry::File { len, mode, .. } => write_stat(stat_ptr, false, len, mode, 2),
        }
    }
}

fn sys_newfstatat(dirfd: isize, path_ptr: *const u8, stat_ptr: *mut u8, flags: usize) -> isize {
    if stat_ptr.is_null() {
        return -(EFAULT as isize);
    }

    if path_ptr.is_null() {
        return -(EFAULT as isize);
    }

    let path = match read_cstr(path_ptr) {
        Some(p) => p,
        None => return -(EFAULT as isize),
    };

    if path.is_empty() && (flags & AT_EMPTY_PATH) != 0 {
        if dirfd == AT_FDCWD {
            return write_stat(stat_ptr, true, 0, 0o040755, 1);
        }
        if dirfd < 0 {
            return -(EBADF as isize);
        }
        return sys_fstat(dirfd as usize, stat_ptr);
    }

    if !is_dirfd_ok(dirfd) {
        return -(EBADF as isize);
    }

    sys_stat_path(path_ptr, stat_ptr)
}

fn sys_getdents64(fd: usize, dirp: *mut u8, count: usize) -> isize {
    if dirp.is_null() {
        return -(EFAULT as isize);
    }

    unsafe {
        if fd >= MAX_FDS {
            return -(EBADF as isize);
        }

        let FdEntry::RootDir { pos } = &mut FD_TABLE[fd] else {
            return -(ENOTDIR as isize);
        };

        let total = root_entries_count();
        if *pos >= total {
            return 0;
        }

        let mut written = 0usize;
        while *pos < total {
            let (name, typ, ino) = match *pos {
                0 => (".", DT_DIR, 1u64),
                1 => ("..", DT_DIR, 1u64),
                idx => {
                    let file_idx = idx - 2;
                    match root_name_by_index(file_idx) {
                        Some(n) => (n, DT_REG, (file_idx + 2) as u64),
                        None => break,
                    }
                }
            };

            let reclen = dirent_record_len(name.len());
            if written + reclen > count {
                if written == 0 {
                    return -(EINVAL as isize);
                }
                break;
            }

            let rec = dirp.add(written);
            write_u64(rec, 0, ino);
            write_i64(rec, 8, (*pos as i64) + 1);
            write_u16(rec, 16, reclen as u16);
            write_u8(rec, 18, typ);

            let name_ptr = rec.add(19);
            ptr::copy_nonoverlapping(name.as_ptr(), name_ptr, name.len());
            write_u8(rec, 19 + name.len(), 0);

            let mut pad = 20 + name.len();
            while pad < reclen {
                write_u8(rec, pad, 0);
                pad += 1;
            }

            written += reclen;
            *pos += 1;
        }

        written as isize
    }
}

fn is_dirfd_ok(dirfd: isize) -> bool {
    if dirfd == AT_FDCWD {
        return true;
    }
    if dirfd < 0 {
        return false;
    }
    let fd = dirfd as usize;
    unsafe {
        if fd >= MAX_FDS {
            return false;
        }
        matches!(FD_TABLE[fd], FdEntry::RootDir { .. })
    }
}

fn normalize_path(path: &str) -> Option<&str> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Some("");
    }

    let p = if let Some(stripped) = trimmed.strip_prefix("./") {
        stripped
    } else {
        trimmed
    };

    let p = p.trim_start_matches('/');
    if p.contains('/') {
        return None;
    }
    Some(p)
}

fn read_cstr(ptr: *const u8) -> Option<&'static str> {
    unsafe {
        let mut len = 0usize;
        while len < CSTR_MAX {
            if ptr.add(len).read() == 0 {
                let bytes = core::slice::from_raw_parts(ptr, len);
                return core::str::from_utf8(bytes).ok();
            }
            len += 1;
        }
    }
    None
}

fn root_entries_count() -> usize {
    if let Ok(fs_image) = rootfs() {
        if let Ok(h) = fs::header(fs_image) {
            return h.file_count as usize + 2;
        }
    }
    2
}

fn root_name_by_index(file_idx: usize) -> Option<&'static str> {
    let fs_image = rootfs().ok()?;
    fs::entry(fs_image, file_idx).ok().map(|rec| rec.name)
}

fn write_stat(stat_ptr: *mut u8, is_dir: bool, size: usize, mode: u16, ino: u64) -> isize {
    let mode_type = if is_dir { 0o040000u32 } else { 0o100000u32 };
    let st = LinuxStat {
        st_dev: 1,
        st_ino: ino,
        st_nlink: 1,
        st_mode: mode_type | (mode as u32 & 0o7777),
        st_uid: 0,
        st_gid: 0,
        __pad0: 0,
        st_rdev: 0,
        st_size: size as i64,
        st_blksize: 4096,
        st_blocks: ((size + 511) / 512) as i64,
        st_atime: 0,
        st_atime_nsec: 0,
        st_mtime: 0,
        st_mtime_nsec: 0,
        st_ctime: 0,
        st_ctime_nsec: 0,
        __unused: [0; 3],
    };

    unsafe {
        ptr::copy_nonoverlapping(
            (&st as *const LinuxStat).cast::<u8>(),
            stat_ptr,
            size_of::<LinuxStat>(),
        );
    }
    0
}

fn dirent_record_len(name_len: usize) -> usize {
    let raw = 19 + name_len + 1;
    (raw + 7) & !7
}

unsafe fn write_u8(base: *mut u8, off: usize, val: u8) {
    base.add(off).write(val);
}

unsafe fn write_u16(base: *mut u8, off: usize, val: u16) {
    ptr::write_unaligned(base.add(off).cast::<u16>(), val);
}

unsafe fn write_u64(base: *mut u8, off: usize, val: u64) {
    ptr::write_unaligned(base.add(off).cast::<u64>(), val);
}

unsafe fn write_i64(base: *mut u8, off: usize, val: i64) {
    ptr::write_unaligned(base.add(off).cast::<i64>(), val);
}


fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    if buf.is_null() || size == 0 {
        return -(EINVAL as isize);
    }
    unsafe {
        if CWD_LEN + 1 > size {
            return -(ERANGE as isize);
        }
        ptr::copy_nonoverlapping(CWD_BUF.as_ptr(), buf, CWD_LEN);
        buf.add(CWD_LEN).write(0);
        buf as isize
    }
}

fn sys_chdir(path_ptr: *const u8) -> isize {
    if path_ptr.is_null() {
        return -(EFAULT as isize);
    }
    let path = match read_cstr(path_ptr) {
        Some(p) => p,
        None => return -(EFAULT as isize),
    };

    if path == "/" || path == "." || path.is_empty() {
        unsafe {
            CWD_BUF[0] = b'/';
            CWD_LEN = 1;
        }
        return 0;
    }

    -(ENOENT as isize)
}

fn sys_access(path_ptr: *const u8) -> isize {
    if path_ptr.is_null() {
        return -(EFAULT as isize);
    }
    match read_cstr(path_ptr) {
        Some(path) if path_exists(path) => 0,
        Some(_) => -(ENOENT as isize),
        None => -(EFAULT as isize),
    }
}

fn sys_faccessat(_dirfd: isize, path_ptr: *const u8) -> isize {
    sys_access(path_ptr)
}

fn sys_readlink(path_ptr: *const u8, buf: *mut u8, bufsz: usize) -> isize {
    if path_ptr.is_null() {
        return -(EFAULT as isize);
    }
    let path = match read_cstr(path_ptr) {
        Some(p) => p,
        None => return -(EFAULT as isize),
    };
    readlink_impl(path, buf, bufsz)
}

fn sys_readlinkat(_dirfd: isize, path_ptr: *const u8, buf: *mut u8, bufsz: usize) -> isize {
    sys_readlink(path_ptr, buf, bufsz)
}

fn readlink_impl(path: &str, buf: *mut u8, bufsz: usize) -> isize {
    if buf.is_null() || bufsz == 0 {
        return -(EINVAL as isize);
    }

    let target = if path == "/proc/self/exe" || path == "proc/self/exe" {
        b"/bin/busybox".as_slice()
    } else {
        return -(ENOENT as isize);
    };

    let n = min(target.len(), bufsz);
    unsafe {
        ptr::copy_nonoverlapping(target.as_ptr(), buf, n);
    }
    n as isize
}

fn sys_uname(buf: *mut u8) -> isize {
    if buf.is_null() {
        return -(EFAULT as isize);
    }

    // struct utsname: 6 fields * 65 bytes
    const FIELD: usize = 65;
    let vals: [&[u8]; 6] = [
        b"LeonOS",
        b"leonos3",
        b"0.1.0",
        b"#1 LeonOS",
        b"x86_64",
        b"",
    ];

    unsafe {
        for i in 0..6usize {
            let base = buf.add(i * FIELD);
            for j in 0..FIELD {
                base.add(j).write(0);
            }
            let s = vals[i];
            let n = min(s.len(), FIELD - 1);
            ptr::copy_nonoverlapping(s.as_ptr(), base, n);
        }
    }

    0
}

fn sys_clock_gettime(_clock_id: usize, tp: *mut u8) -> isize {
    if tp.is_null() {
        return -(EFAULT as isize);
    }
    unsafe {
        write_i64(tp, 0, 0);
        write_i64(tp, 8, 0);
    }
    0
}

fn sys_gettimeofday(tv: *mut u8, _tz: *mut u8) -> isize {
    if tv.is_null() {
        return 0;
    }
    unsafe {
        write_i64(tv, 0, 0);
        write_i64(tv, 8, 0);
    }
    0
}

fn sys_getrandom(buf: *mut u8, len: usize) -> isize {
    if buf.is_null() {
        return -(EFAULT as isize);
    }

    // Deterministic placeholder random stream.
    unsafe {
        let mut x: u8 = 0xA7;
        for i in 0..len {
            x = x.wrapping_mul(37).wrapping_add(17);
            buf.add(i).write(x);
        }
    }

    len as isize
}

fn sys_dup(fd: usize) -> isize {
    if fd <= 2 {
        return fd as isize;
    }
    unsafe {
        if fd >= MAX_FDS {
            return -(EBADF as isize);
        }
        match FD_TABLE[fd] {
            FdEntry::Free => -(EBADF as isize),
            entry => alloc_fd(entry),
        }
    }
}

fn sys_dup2(oldfd: usize, newfd: usize) -> isize {
    if oldfd <= 2 {
        if newfd <= 2 {
            return newfd as isize;
        }
        return -(EBADF as isize);
    }

    unsafe {
        if oldfd >= MAX_FDS || newfd >= MAX_FDS || newfd < FD_BASE {
            return -(EBADF as isize);
        }

        let src = match FD_TABLE[oldfd] {
            FdEntry::Free => return -(EBADF as isize),
            e => e,
        };

        FD_TABLE[newfd] = src;
        newfd as isize
    }
}

fn path_exists(path: &str) -> bool {
    if path == "/" || path == "." || path.is_empty() {
        return true;
    }

    let Some(name) = normalize_path(path) else {
        return false;
    };

    if name.is_empty() {
        return true;
    }

    let Ok(fs_image) = rootfs() else {
        return false;
    };

    fs::open(fs_image, name).is_ok()
}

pub fn set_stdin_reader(reader: fn() -> Option<u8>) {
    posix::set_stdin_reader(reader);
}

pub fn read_stdin_byte_blocking() -> Option<u8> {
    posix::read_stdin_byte_blocking()
}
pub fn last_exit_code() -> i32 {
    LAST_EXIT.load(Ordering::Relaxed)
}

pub fn posix_snapshot() -> (u32, usize) {
    (POSIX.getpid(), POSIX.brk_get())
}

pub fn supported_syscalls_hint() -> &'static str {
    "read, write, close, dup/dup2, brk, mmap, mprotect, munmap, rt_sigaction, rt_sigprocmask, ioctl, getpid/gettid, exit/exit_group, arch_prctl, futex, clock_gettime/gettimeofday, uname, uid/gid, getcwd/chdir, access/faccessat, readlink/readlinkat, openat, fstat/newfstatat, lseek, getdents64, getrandom, prlimit64, set_robust_list"
}






