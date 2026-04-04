// Freestanding Linux x86_64 userspace test (no libc).
// Test syscalls: read(0), write(1), exit(60).

typedef unsigned long usize;
typedef long isize;

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

static inline isize sys_read(int fd, void* buf, usize len) {
    return sys_call3(0, (usize)fd, (usize)buf, len);
}

static inline void sys_exit(int code) {
    (void)sys_call3(60, (usize)code, 0, 0);
    for (;;) {
        __asm__ volatile ("hlt");
    }
}

void _start(void) {
    static const char prompt[] = "What is your name? ";
    static const char hello[] = "Hello, ";
    static const char fallback[] = "friend";
    static const char tail[] = "!\\n";

    char buf[64];
    isize n;

    (void)sys_write(1, prompt, sizeof(prompt) - 1);
    n = sys_read(0, buf, sizeof(buf) - 1);

    if (n < 0) {
        n = 0;
    }

    while (n > 0 && (buf[n - 1] == '\n' || buf[n - 1] == '\r')) {
        n--;
    }

    (void)sys_write(1, hello, sizeof(hello) - 1);
    if (n > 0) {
        (void)sys_write(1, buf, (usize)n);
    } else {
        (void)sys_write(1, fallback, sizeof(fallback) - 1);
    }
    (void)sys_write(1, tail, sizeof(tail) - 1);

    sys_exit(0);
}
