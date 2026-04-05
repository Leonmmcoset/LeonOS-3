// Reap last exit status from LeonOS single-process wait4 model.

typedef unsigned long usize;
typedef long isize;

static inline isize sys_call4(usize n, usize a0, usize a1, usize a2, usize a3) {
    isize ret;
    register usize r10 __asm__("r10") = a3;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(n), "D"(a0), "S"(a1), "d"(a2), "r"(r10)
        : "rcx", "r11", "memory"
    );
    return ret;
}

static inline isize sys_call3(usize n, usize a0, usize a1, usize a2) {
    isize ret;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(n), "D"(a0), "S"(a1), "d"(a2)
        : "rcx", "r11", "memory"
    );
    return ret;
}

static inline isize sys_write(int fd, const void* buf, usize len) {
    return sys_call3(1, (usize)fd, (usize)buf, len);
}

static inline isize sys_wait4(isize pid, int* status, int options, void* rusage) {
    return sys_call4(61, (usize)pid, (usize)status, (usize)options, (usize)rusage);
}

static inline void sys_exit(int code) {
    (void)sys_call3(60, (usize)code, 0, 0);
    for (;;) {
        __asm__ volatile ("hlt");
    }
}

static usize append_str(char* out, usize pos, const char* s) {
    usize i = 0;
    while (s[i] != '\0') {
        out[pos++] = s[i++];
    }
    return pos;
}

static usize append_i64(char* out, usize pos, long v) {
    char tmp[32];
    usize n = 0;
    unsigned long x;

    if (v < 0) {
        out[pos++] = '-';
        x = (unsigned long)(-v);
    } else {
        x = (unsigned long)v;
    }

    if (x == 0) {
        out[pos++] = '0';
        return pos;
    }

    while (x > 0) {
        tmp[n++] = (char)('0' + (x % 10));
        x /= 10;
    }

    while (n > 0) {
        out[pos++] = tmp[--n];
    }
    return pos;
}

void _start(void) {
    int status = 0;
    isize ret = sys_wait4(-1, &status, 0, (void*)0);

    int exited = ((status & 0x7f) == 0);
    int code = (status >> 8) & 0xff;

    char line[200];
    usize p = 0;
    p = append_str(line, p, "wait4_reap: ret=");
    p = append_i64(line, p, (long)ret);
    p = append_str(line, p, ", raw_status=");
    p = append_i64(line, p, (long)status);
    p = append_str(line, p, ", exited=");
    p = append_i64(line, p, (long)exited);
    p = append_str(line, p, ", code=");
    p = append_i64(line, p, (long)code);
    p = append_str(line, p, "\n");
    (void)sys_write(1, line, p);

    if (ret >= 0 && exited && code == 7) {
        sys_exit(0);
    }
    if (ret == -10) {
        // No pending child state.
        sys_exit(2);
    }
    sys_exit(1);
}
