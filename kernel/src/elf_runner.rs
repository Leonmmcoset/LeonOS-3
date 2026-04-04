extern crate alloc;

use alloc::vec;
use os_terminal::{DrawTarget, Terminal};

use crate::elf::{self, ElfError};
use crate::syscall;

#[derive(Debug)]
pub enum RunError {
    Parse(ElfError),
    NoLoadSegment,
    AddressOutOfRange,
    UnsupportedComplexElf {
        entry: u64,
        segments: usize,
        memory_size: usize,
    },
    UnsupportedInstruction {
        rip: u64,
        bytes: [u8; 8],
    },
    StepLimit {
        steps: usize,
        last_rip: u64,
    },
}

struct Cpu {
    rax: u64,
    rbp: u64,
    rsp: u64,
    rdi: u64,
    rsi: u64,
    rdx: u64,
    rcx: u64,
    r8: u64,
    r9: u64,
    rip: u64,
    zf: bool,
    sf: bool,
}

const MAX_STEPS: usize = 128 * 1024;
const MAX_INTERP_MEM: usize = 16 * 1024 * 1024;

pub fn run_linux_elf<D: DrawTarget>(terminal: &mut Terminal<D>, image: &[u8]) -> Result<(), RunError> {
    let elf = elf::parse_elf64(image).map_err(RunError::Parse)?;
    if elf.segments.is_empty() {
        return Err(RunError::NoLoadSegment);
    }

    let mut min_vaddr = u64::MAX;
    let mut max_vaddr = 0u64;
    for seg in &elf.segments {
        min_vaddr = min_vaddr.min(seg.vaddr);
        max_vaddr = max_vaddr.max(seg.vaddr.saturating_add(seg.mem_size));
    }

    if max_vaddr <= min_vaddr {
        return Err(RunError::AddressOutOfRange);
    }

    let mem_len = (max_vaddr - min_vaddr) as usize;
    if mem_len > MAX_INTERP_MEM || elf.segments.len() > 16 {
        return Err(RunError::UnsupportedComplexElf {
            entry: elf.entry,
            segments: elf.segments.len(),
            memory_size: mem_len,
        });
    }

    let mut mem = vec![0u8; mem_len];

    for seg in &elf.segments {
        let dst_start = (seg.vaddr - min_vaddr) as usize;
        let dst_end = dst_start.saturating_add(seg.mem_size as usize);
        if dst_end > mem.len() {
            return Err(RunError::AddressOutOfRange);
        }

        let src_start = seg.file_offset as usize;
        let src_end = src_start.saturating_add(seg.file_size as usize);
        if src_end > image.len() {
            return Err(RunError::AddressOutOfRange);
        }

        let file_len = seg.file_size as usize;
        mem[dst_start..dst_start + file_len].copy_from_slice(&image[src_start..src_end]);
    }

    let stack_top = min_vaddr + (mem.len() as u64) - 0x10;
    let mut cpu = Cpu {
        rax: 0,
        rbp: 0,
        rsp: stack_top,
        rdi: 0,
        rsi: 0,
        rdx: 0,
        rcx: 0,
        r8: 0,
        r9: 0,
        rip: elf.entry,
        zf: false,
        sf: false,
    };

    for _ in 0..MAX_STEPS {
        if exec_one(&mut cpu, min_vaddr, &mut mem, terminal)? {
            return Ok(());
        }
    }

    Err(RunError::StepLimit {
        steps: MAX_STEPS,
        last_rip: cpu.rip,
    })
}

fn exec_one<D: DrawTarget>(
    cpu: &mut Cpu,
    base: u64,
    mem: &mut [u8],
    terminal: &mut Terminal<D>,
) -> Result<bool, RunError> {
    let b = read_bytes(base, mem, cpu.rip)?;

    // ENDBR64 (CET) as NOP.
    if b[0] == 0xF3 && b[1] == 0x0F && b[2] == 0x1E && b[3] == 0xFA {
        cpu.rip += 4;
        return Ok(false);
    }

    if b[0] == 0x90 {
        cpu.rip += 1;
        return Ok(false);
    }

    if b[0] == 0xC3 || b[0] == 0xF4 {
        return Ok(true);
    }

    // mov rdi, rax
    if b[0] == 0x48 && b[1] == 0x89 && b[2] == 0xC7 {
        cpu.rdi = cpu.rax;
        cpu.rip += 3;
        return Ok(false);
    }

    // mov rax, rdi
    if b[0] == 0x48 && b[1] == 0x89 && b[2] == 0xF8 {
        cpu.rax = cpu.rdi;
        cpu.rip += 3;
        return Ok(false);
    }

    // mov rdx, r8
    if b[0] == 0x4C && b[1] == 0x89 && b[2] == 0xC2 {
        cpu.rdx = cpu.r8;
        cpu.rip += 3;
        return Ok(false);
    }

    // mov rsi, r9
    if b[0] == 0x4C && b[1] == 0x89 && b[2] == 0xCE {
        cpu.rsi = cpu.r9;
        cpu.rip += 3;
        return Ok(false);
    }

    // mov rdi, rdx
    if b[0] == 0x48 && b[1] == 0x89 && b[2] == 0xD7 {
        cpu.rdi = cpu.rdx;
        cpu.rip += 3;
        return Ok(false);
    }

    // mov rsi, rdx
    if b[0] == 0x48 && b[1] == 0x89 && b[2] == 0xD6 {
        cpu.rsi = cpu.rdx;
        cpu.rip += 3;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0x31 && b[2] == 0xED {
        cpu.rbp = 0;
        cpu.rip += 3;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0x89 && b[2] == 0xE7 {
        cpu.rdi = cpu.rsp;
        cpu.rip += 3;
        return Ok(false);
    }

    // mov rsi, [rdi]
    if b[0] == 0x48 && b[1] == 0x8B && b[2] == 0x37 {
        cpu.rsi = read_u64(base, mem, cpu.rdi)?;
        cpu.rip += 3;
        return Ok(false);
    }

    // lea rdx, [rdi + 8]
    if b[0] == 0x48 && b[1] == 0x8D && b[2] == 0x57 && b[3] == 0x08 {
        cpu.rdx = cpu.rdi.wrapping_add(8);
        cpu.rip += 4;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0x89 && b[2] == 0xE2 {
        cpu.rdx = cpu.rsp;
        cpu.rip += 3;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0x89 && b[2] == 0xE6 {
        cpu.rsi = cpu.rsp;
        cpu.rip += 3;
        return Ok(false);
    }

    if b[0] == 0x49 && b[1] == 0x89 && b[2] == 0xD1 {
        cpu.r9 = cpu.rdx;
        cpu.rip += 3;
        return Ok(false);
    }

    if b[0] == 0x49 && b[1] == 0x89 && b[2] == 0xD0 {
        cpu.r8 = cpu.rdx;
        cpu.rip += 3;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0x83 && b[2] == 0xE4 {
        let mask = (b[3] as i8 as i64) as u64;
        cpu.rsp &= mask;
        cpu.rip += 4;
        return Ok(false);
    }

    // lea r9, [rsp-0x40]
    if b[0] == 0x4C && b[1] == 0x8D && b[2] == 0x4C && b[3] == 0x24 {
        let disp = b[4] as i8 as i64;
        cpu.r9 = ((cpu.rsp as i64) + disp) as u64;
        cpu.rip += 5;
        return Ok(false);
    }

    if b[0] == 0x50 {
        push_u64(base, mem, cpu, cpu.rax)?;
        cpu.rip += 1;
        return Ok(false);
    }

    if b[0] == 0x54 {
        push_u64(base, mem, cpu, cpu.rsp)?;
        cpu.rip += 1;
        return Ok(false);
    }

    if b[0] == 0x5E {
        cpu.rsi = pop_u64(base, mem, cpu)?;
        cpu.rip += 1;
        return Ok(false);
    }

    if b[0] == 0x45 && b[1] == 0x31 && b[2] == 0xC0 {
        cpu.r8 = 0;
        cpu.rip += 3;
        return Ok(false);
    }

    if b[0] == 0x45 && b[1] == 0x31 && b[2] == 0xC9 {
        cpu.r9 = 0;
        cpu.rip += 3;
        return Ok(false);
    }

    if b[0] == 0x31 && b[1] == 0xC9 {
        cpu.rcx = 0;
        cpu.rip += 2;
        return Ok(false);
    }

    if b[0] == 0x31 && b[1] == 0xD2 {
        cpu.rdx = 0;
        cpu.rip += 2;
        return Ok(false);
    }

    if b[0] == 0x31 && b[1] == 0xC0 {
        cpu.rax = 0;
        cpu.rip += 2;
        return Ok(false);
    }

    if b[0] == 0x31 && b[1] == 0xFF {
        cpu.rdi = 0;
        cpu.rip += 2;
        return Ok(false);
    }

    // test rax, rax
    if b[0] == 0x48 && b[1] == 0x85 && b[2] == 0xC0 {
        let v = cpu.rax;
        cpu.zf = v == 0;
        cpu.sf = (v >> 63) != 0;
        cpu.rip += 3;
        return Ok(false);
    }

    // test rdx, rdx
    if b[0] == 0x48 && b[1] == 0x85 && b[2] == 0xD2 {
        let v = cpu.rdx;
        cpu.zf = v == 0;
        cpu.sf = (v >> 63) != 0;
        cpu.rip += 3;
        return Ok(false);
    }

    // cmovns rdx, rax
    if b[0] == 0x48 && b[1] == 0x0F && b[2] == 0x49 && b[3] == 0xD0 {
        if !cpu.sf {
            cpu.rdx = cpu.rax;
        }
        cpu.rip += 4;
        return Ok(false);
    }

    // mov al, [r9 + r8 - 1]
    if b[0] == 0x43 && b[1] == 0x8A && b[2] == 0x44 && b[3] == 0x01 && b[4] == 0xFF {
        let addr = cpu.r9.wrapping_add(cpu.r8).wrapping_sub(1);
        let v = read_u8(base, mem, addr)?;
        set_al(cpu, v);
        cpu.rip += 5;
        return Ok(false);
    }

    // lea rdx, [rdx - 1]
    if b[0] == 0x48 && b[1] == 0x8D && b[2] == 0x52 && b[3] == 0xFF {
        cpu.rdx = cpu.rdx.wrapping_sub(1);
        cpu.rip += 4;
        return Ok(false);
    }

    // cmp al, imm8
    if b[0] == 0x3C {
        let lhs = get_al(cpu);
        let rhs = b[1];
        cpu.zf = lhs == rhs;
        cpu.sf = ((lhs as i8).wrapping_sub(rhs as i8)) < 0;
        cpu.rip += 2;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0xC7 && b[2] == 0xC0 {
        cpu.rax = read_u32(base, mem, cpu.rip + 3)? as u64;
        cpu.rip += 7;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0xC7 && b[2] == 0xC7 {
        cpu.rdi = read_u32(base, mem, cpu.rip + 3)? as u64;
        cpu.rip += 7;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0xC7 && b[2] == 0xC6 {
        cpu.rsi = read_u32(base, mem, cpu.rip + 3)? as u64;
        cpu.rip += 7;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0xC7 && b[2] == 0xC2 {
        cpu.rdx = read_u32(base, mem, cpu.rip + 3)? as u64;
        cpu.rip += 7;
        return Ok(false);
    }

    // mov r8d/r9d, imm32
    if b[0] == 0x41 && (0xB8..=0xBF).contains(&b[1]) {
        let imm = read_u32(base, mem, cpu.rip + 2)? as u64;
        match b[1] - 0xB8 {
            0 => cpu.r8 = imm,
            1 => cpu.r9 = imm,
            _ => {}
        }
        cpu.rip += 6;
        return Ok(false);
    }

    // mov r32, imm32 (zero-extend to 64-bit)
    if (0xB8..=0xBF).contains(&b[0]) {
        let imm = read_u32(base, mem, cpu.rip + 1)? as u64;
        match b[0] - 0xB8 {
            0 => cpu.rax = imm,
            1 => cpu.rcx = imm,
            2 => cpu.rdx = imm,
            3 => {}
            4 => cpu.rsp = imm,
            5 => cpu.rbp = imm,
            6 => cpu.rsi = imm,
            7 => cpu.rdi = imm,
            _ => {}
        }
        cpu.rip += 5;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0x8D && b[2] == 0x3D {
        let disp = read_i32(base, mem, cpu.rip + 3)? as i64;
        let next = cpu.rip + 7;
        cpu.rdi = ((next as i64) + disp) as u64;
        cpu.rip = next;
        return Ok(false);
    }

    if b[0] == 0x4C && b[1] == 0x8D && b[2] == 0x05 {
        let disp = read_i32(base, mem, cpu.rip + 3)? as i64;
        let next = cpu.rip + 7;
        cpu.r8 = ((next as i64) + disp) as u64;
        cpu.rip = next;
        return Ok(false);
    }

    if b[0] == 0x4C && b[1] == 0x8D && b[2] == 0x0D {
        let disp = read_i32(base, mem, cpu.rip + 3)? as i64;
        let next = cpu.rip + 7;
        cpu.r9 = ((next as i64) + disp) as u64;
        cpu.rip = next;
        return Ok(false);
    }

    if b[0] == 0x48 && b[1] == 0x8D && b[2] == 0x35 {
        let disp = read_i32(base, mem, cpu.rip + 3)? as i64;
        let next = cpu.rip + 7;
        cpu.rsi = ((next as i64) + disp) as u64;
        cpu.rip = next;
        return Ok(false);
    }

    if b[0] == 0x0F && b[1] == 0x05 {
        let exit = handle_syscall(cpu, base, mem, terminal)?;
        cpu.rip += 2;
        return Ok(exit);
    }

    if b[0] == 0xE8 {
        let rel = read_i32(base, mem, cpu.rip + 1)? as i64;
        let next = cpu.rip + 5;
        push_u64(base, mem, cpu, next)?;
        cpu.rip = ((next as i64) + rel) as u64;
        return Ok(false);
    }

    if b[0] == 0xE9 {
        let rel = read_i32(base, mem, cpu.rip + 1)? as i64;
        let next = cpu.rip + 5;
        cpu.rip = ((next as i64) + rel) as u64;
        return Ok(false);
    }

    // short jump (unconditional)
    if b[0] == 0xEB {
        let rel = b[1] as i8 as i64;
        let next = cpu.rip + 2;
        cpu.rip = ((next as i64) + rel) as u64;
        return Ok(false);
    }

    // short JE/JZ
    if b[0] == 0x74 {
        let rel = b[1] as i8 as i64;
        let next = cpu.rip + 2;
        if cpu.zf {
            cpu.rip = ((next as i64) + rel) as u64;
        } else {
            cpu.rip = next;
        }
        return Ok(false);
    }

    if b[0] == 0xFF && b[1] == 0x15 {
        let disp = read_i32(base, mem, cpu.rip + 2)? as i64;
        let next = cpu.rip + 6;
        let ptr_va = ((next as i64) + disp) as u64;
        let target = read_u64(base, mem, ptr_va)?;

        if target < base || target >= (base + mem.len() as u64) {
            if cpu.rdi >= base && cpu.rdi < (base + mem.len() as u64) {
                cpu.rip = cpu.rdi;
                return Ok(false);
            }
        }

        push_u64(base, mem, cpu, next)?;
        cpu.rip = target;
        return Ok(false);
    }

    Err(RunError::UnsupportedInstruction {
        rip: cpu.rip,
        bytes: b,
    })
}

fn handle_syscall<D: DrawTarget>(
    cpu: &mut Cpu,
    base: u64,
    mem: &mut [u8],
    terminal: &mut Terminal<D>,
) -> Result<bool, RunError> {
    match cpu.rax as usize {
        syscall::SYS_READ => {
            let fd = cpu.rdi as usize;
            let len = cpu.rdx as usize;
            let start = va_to_index(base, mem, cpu.rsi)?;
            let cap = core::cmp::min(len, mem.len() - start);
            if cap == 0 {
                cpu.rax = 0;
                return Ok(false);
            }

            if fd == 0 {
                let mut got = 0usize;
                while got < cap {
                    let Some(ch) = syscall::read_stdin_byte_blocking() else {
                        break;
                    };
                    mem[start + got] = ch;
                    terminal.process(&[ch]);
                    got += 1;
                    if ch == b'\n' || ch == b'\r' {
                        break;
                    }
                }
                cpu.rax = got as u64;
                return Ok(false);
            }

            let mut tmp = [0u8; 256];
            let want = core::cmp::min(cap, tmp.len());
            let n = syscall::dispatch(
                syscall::SYS_READ,
                fd,
                tmp.as_mut_ptr() as usize,
                want,
                0,
                0,
                0,
            );

            if n >= 0 {
                let got = n as usize;
                mem[start..start + got].copy_from_slice(&tmp[..got]);
            }
            cpu.rax = n as u64;
            Ok(false)
        }
        syscall::SYS_WRITE => {
            let fd = cpu.rdi as usize;
            let len = cpu.rdx as usize;
            let start = va_to_index(base, mem, cpu.rsi)?;
            let end = start.checked_add(len).ok_or(RunError::AddressOutOfRange)?;
            if end > mem.len() {
                return Err(RunError::AddressOutOfRange);
            }
            if fd == 1 || fd == 2 {
                terminal.process(&mem[start..end]);
            }
            cpu.rax = syscall::dispatch(syscall::SYS_WRITE, fd, cpu.rsi as usize, len, 0, 0, 0) as u64;
            Ok(false)
        }
        syscall::SYS_GETPID => {
            cpu.rax = syscall::dispatch(syscall::SYS_GETPID, 0, 0, 0, 0, 0, 0) as u64;
            Ok(false)
        }
        syscall::SYS_BRK => {
            cpu.rax = syscall::dispatch(syscall::SYS_BRK, cpu.rdi as usize, 0, 0, 0, 0, 0) as u64;
            Ok(false)
        }
        syscall::SYS_EXIT | syscall::SYS_EXIT_GROUP => {
            cpu.rax = syscall::dispatch(syscall::SYS_EXIT, cpu.rdi as usize, 0, 0, 0, 0, 0) as u64;
            Ok(true)
        }
        nr => {
            cpu.rax = syscall::dispatch(nr, cpu.rdi as usize, cpu.rsi as usize, cpu.rdx as usize, 0, 0, 0) as u64;
            Ok(false)
        }
    }
}

#[inline]
fn get_al(cpu: &Cpu) -> u8 {
    (cpu.rax & 0xff) as u8
}

#[inline]
fn set_al(cpu: &mut Cpu, v: u8) {
    cpu.rax = (cpu.rax & !0xff) | (v as u64);
}

fn push_u64(base: u64, mem: &mut [u8], cpu: &mut Cpu, v: u64) -> Result<(), RunError> {
    let new_rsp = cpu.rsp.checked_sub(8).ok_or(RunError::AddressOutOfRange)?;
    write_u64(base, mem, new_rsp, v)?;
    cpu.rsp = new_rsp;
    Ok(())
}

fn pop_u64(base: u64, mem: &mut [u8], cpu: &mut Cpu) -> Result<u64, RunError> {
    let v = read_u64(base, mem, cpu.rsp)?;
    cpu.rsp = cpu.rsp.checked_add(8).ok_or(RunError::AddressOutOfRange)?;
    Ok(v)
}

fn va_to_index(base: u64, mem: &[u8], va: u64) -> Result<usize, RunError> {
    if va < base {
        return Err(RunError::AddressOutOfRange);
    }
    let idx = (va - base) as usize;
    if idx >= mem.len() {
        return Err(RunError::AddressOutOfRange);
    }
    Ok(idx)
}

fn read_bytes(base: u64, mem: &[u8], va: u64) -> Result<[u8; 8], RunError> {
    let idx = va_to_index(base, mem, va)?;
    let mut out = [0u8; 8];
    let n = core::cmp::min(8, mem.len() - idx);
    out[..n].copy_from_slice(&mem[idx..idx + n]);
    Ok(out)
}

fn read_u8(base: u64, mem: &[u8], va: u64) -> Result<u8, RunError> {
    let i = va_to_index(base, mem, va)?;
    Ok(mem[i])
}

fn read_u32(base: u64, mem: &[u8], va: u64) -> Result<u32, RunError> {
    let i = va_to_index(base, mem, va)?;
    let end = i.checked_add(4).ok_or(RunError::AddressOutOfRange)?;
    if end > mem.len() {
        return Err(RunError::AddressOutOfRange);
    }
    Ok(u32::from_le_bytes([mem[i], mem[i + 1], mem[i + 2], mem[i + 3]]))
}

fn read_u64(base: u64, mem: &[u8], va: u64) -> Result<u64, RunError> {
    let i = va_to_index(base, mem, va)?;
    let end = i.checked_add(8).ok_or(RunError::AddressOutOfRange)?;
    if end > mem.len() {
        return Err(RunError::AddressOutOfRange);
    }
    Ok(u64::from_le_bytes([
        mem[i],
        mem[i + 1],
        mem[i + 2],
        mem[i + 3],
        mem[i + 4],
        mem[i + 5],
        mem[i + 6],
        mem[i + 7],
    ]))
}

fn write_u64(base: u64, mem: &mut [u8], va: u64, v: u64) -> Result<(), RunError> {
    let i = va_to_index(base, mem, va)?;
    let end = i.checked_add(8).ok_or(RunError::AddressOutOfRange)?;
    if end > mem.len() {
        return Err(RunError::AddressOutOfRange);
    }
    mem[i..end].copy_from_slice(&v.to_le_bytes());
    Ok(())
}

fn read_i32(base: u64, mem: &[u8], va: u64) -> Result<i32, RunError> {
    Ok(read_u32(base, mem, va)? as i32)
}





