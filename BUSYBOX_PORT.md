# LeonOS 3 BusyBox Port Status

## Current State

LeonOS 3 now has a broader Linux syscall compatibility surface aimed at BusyBox startup/probing:

- file and directory fd table over rootfs ramdisk
- `openat`, `read`, `close`, `lseek`
- `newfstatat`, `fstat`, `stat`, `lstat`
- `getdents64`
- `dup`, `dup2`
- `getcwd`, `chdir`
- `access`, `faccessat`
- `readlink`, `readlinkat` (`/proc/self/exe` placeholder)
- `uname`
- `clock_gettime`, `gettimeofday`
- `getrandom`

## Important Limitation

BusyBox still cannot run fully yet because LeonOS 3 does not yet implement:

- real userspace process context switching
- real `execve` and argv/envp stack setup
- proper VFS (nested dirs, devices, procfs)
- pipe/fork/wait signal model
- TTY and `ioctl` behaviors used by interactive applets

## Suggested Next Steps

1. Implement minimal `execve` path that can load ELF into a real userspace address space and jump to ring3.
2. Add minimal `pipe`, `wait4`, and `vfork/fork` shim model for shell scripts.
3. Add `/dev` and `/proc/self` virtual files.
4. Expand `fcntl/ioctl` for TTY flags and non-blocking mode.
5. Add `mount`-style VFS abstraction so rootfs is not flat-only.
