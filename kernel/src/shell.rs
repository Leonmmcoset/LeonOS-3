use core::str;
use os_terminal::{DrawTarget, Terminal};

pub fn run_shell<D, F, H>(terminal: &mut Terminal<D>, mut read_input: F, mut handle_custom: H) -> !
where
    D: DrawTarget,
    F: FnMut() -> Option<u8>,
    H: FnMut(&mut Terminal<D>, &str) -> bool,
{
    let mut line = [0u8; 256];
    let mut len = 0usize;

    terminal.process(b"leon> ");

    loop {
        if let Some(byte) = read_input() {
            match byte {
                b'\r' | b'\n' => {
                    terminal.process(b"\n");

                    if let Ok(input) = str::from_utf8(&line[..len]) {
                        exec_command(terminal, input, &mut handle_custom);
                    } else {
                        terminal.process(b"invalid utf-8 input\n");
                    }

                    len = 0;
                    terminal.process(b"leon> ");
                }
                8 | 127 => {
                    if len > 0 {
                        len -= 1;
                        terminal.process(b"\x08 \x08");
                    }
                }
                0x20..=0x7e => {
                    if len < line.len() {
                        line[len] = byte;
                        len += 1;
                        terminal.process(&[byte]);
                    }
                }
                _ => {}
            }
        } else {
            core::hint::spin_loop();
        }
    }
}

fn exec_command<D, H>(terminal: &mut Terminal<D>, input: &str, handle_custom: &mut H)
where
    D: DrawTarget,
    H: FnMut(&mut Terminal<D>, &str) -> bool,
{
    let cmd = input.trim();

    if cmd.is_empty() {
        return;
    }

    if cmd == "help" {
        terminal.process(b"commands: help, echo, clear, about, ramdisk, ls, stat <f>, cat <f>, posix, syscall, syscap, busybox, elf [f], run <f> [args], runelf, halt\n");
        return;
    }

    if cmd == "about" {
        terminal.process(b"LeonOS 3 - Rust toy OS with os-terminal\n");
        return;
    }

    if cmd == "clear" {
        terminal.process(b"\x1b[2J\x1b[H");
        return;
    }

    if cmd == "halt" {
        terminal.process(b"System halted.\n");
        loop {
            core::hint::spin_loop();
        }
    }

    if let Some(rest) = cmd.strip_prefix("echo ") {
        terminal.process(rest.as_bytes());
        terminal.process(b"\n");
        return;
    }

    if cmd == "echo" {
        terminal.process(b"\n");
        return;
    }

    if handle_custom(terminal, cmd) {
        return;
    }

    terminal.process(b"unknown command: ");
    terminal.process(cmd.as_bytes());
    terminal.process(b"\n");
}


