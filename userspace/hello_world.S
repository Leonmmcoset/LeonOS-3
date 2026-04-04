.section .text
.global _start

_start:
    # write(1, msg, msg_len)
    mov $1, %rax
    mov $1, %rdi
    lea msg(%rip), %rsi
    mov $msg_len, %rdx
    syscall

    # exit(0)
    mov $60, %rax
    xor %rdi, %rdi
    syscall

.section .rodata
msg:
    .ascii "Hello, LeonOS 3!\n"
.set msg_len, . - msg
