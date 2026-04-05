// Exit with code 7 for wait4 test.

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

static inline void sys_exit(int code) {
    (void)sys_call3(60, (usize)code, 0, 0);
    for (;;) {
        __asm__ volatile ("hlt");
    }
}

void _start(void) {
    static const char msg[] = "exit7: exiting with code 7\\n";
    (void)sys_write(1, msg, sizeof(msg) - 1);
    sys_exit(7);
}
