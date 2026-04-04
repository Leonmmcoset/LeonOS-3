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
    InvalidInstruction,
    StepLimit,
}

struct Cpu {
    rax: u64,
    rdi: u64,
    rsi: u64,
    rdx: u64,
    rip: u64,
}

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

    let mut cpu = Cpu {
        rax: 0,
        rdi: 0,
        rsi: 0,
        rdx: 0,
        rip: elf.entry,
    };

    for _ in 0..4096 {
        if exec_one(&mut cpu, min_vaddr, &mem, terminal)? {
            return Ok(());
        }
    }

    Err(RunError::StepLimit)
}

fn exec_one<D: DrawTarget>(
    cpu: &mut Cpu,
    base: u64,
    mem: &[u8],
    terminal: &mut Terminal<D>,
) -> Result<bool, RunError> {
    let b0 = read_u8(base, mem, cpu.rip)?;
    let b1 = read_u8(base, mem, cpu.rip + 1)?;
    let b2 = read_u8(base, mem, cpu.rip + 2)?;

    // mov rax, imm32
    if b0 == 0x48 && b1 == 0xC7 && b2 == 0xC0 {
        cpu.rax = read_u32(base, mem, cpu.rip + 3)? as u64;
        cpu.rip += 7;
        return Ok(false);
    }

    // mov rdi, imm32
    if b0 == 0x48 && b1 == 0xC7 && b2 == 0xC7 {
        cpu.rdi = read_u32(base, mem, cpu.rip + 3)? as u64;
        cpu.rip += 7;
        return Ok(false);
    }

    // mov rdx, imm32
    if b0 == 0x48 && b1 == 0xC7 && b2 == 0xC2 {
        cpu.rdx = read_u32(base, mem, cpu.rip + 3)? as u64;
        cpu.rip += 7;
        return Ok(false);
    }

    // lea rsi, [rip + disp32]
    if b0 == 0x48 && b1 == 0x8D && b2 == 0x35 {
        let disp = read_i32(base, mem, cpu.rip + 3)? as i64;
        let next = cpu.rip + 7;
        cpu.rsi = ((next as i64) + disp) as u64;
        cpu.rip = next;
        return Ok(false);
    }

    // xor rdi, rdi
    if b0 == 0x48 && b1 == 0x31 && b2 == 0xFF {
        cpu.rdi = 0;
        cpu.rip += 3;
        return Ok(false);
    }

    // syscall
    if b0 == 0x0F && b1 == 0x05 {
        let exit = handle_syscall(cpu, base, mem, terminal)?;
        cpu.rip += 2;
        return Ok(exit);
    }

    Err(RunError::InvalidInstruction)
}

fn handle_syscall<D: DrawTarget>(
    cpu: &mut Cpu,
    base: u64,
    mem: &[u8],
    terminal: &mut Terminal<D>,
) -> Result<bool, RunError> {
    match cpu.rax as usize {
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
        syscall::SYS_EXIT => {
            cpu.rax = syscall::dispatch(syscall::SYS_EXIT, cpu.rdi as usize, 0, 0, 0, 0, 0) as u64;
            Ok(true)
        }
        nr => {
            cpu.rax = syscall::dispatch(nr, cpu.rdi as usize, cpu.rsi as usize, cpu.rdx as usize, 0, 0, 0) as u64;
            Ok(false)
        }
    }
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

fn read_u8(base: u64, mem: &[u8], va: u64) -> Result<u8, RunError> {
    let idx = va_to_index(base, mem, va)?;
    Ok(mem[idx])
}

fn read_u32(base: u64, mem: &[u8], va: u64) -> Result<u32, RunError> {
    let i = va_to_index(base, mem, va)?;
    let end = i.checked_add(4).ok_or(RunError::AddressOutOfRange)?;
    if end > mem.len() {
        return Err(RunError::AddressOutOfRange);
    }
    Ok(u32::from_le_bytes([mem[i], mem[i + 1], mem[i + 2], mem[i + 3]]))
}

fn read_i32(base: u64, mem: &[u8], va: u64) -> Result<i32, RunError> {
    Ok(read_u32(base, mem, va)? as i32)
}

