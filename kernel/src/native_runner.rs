extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::arch::global_asm;
use core::fmt::Write as _;
use os_terminal::{DrawTarget, Terminal};
use x86_64::registers::control::Cr3;
use x86_64::registers::model_specific::{Efer, EferFlags, LStar, SFMask, Star};
use x86_64::registers::rflags::RFlags;
use x86_64::structures::paging::{
    Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, Size4KiB, Translate,
};
use x86_64::structures::paging::mapper::TranslateResult;
use x86_64::VirtAddr;

use crate::elf::{self, ElfError};
use crate::logger::serial_write;
use crate::syscall;

#[derive(Debug)]
pub enum NativeError {
    Parse(ElfError),
    NoLoadSegment,
    AddressOutOfRange,
    FixedVaddrUnsupported {
        min_vaddr: u64,
        e_type: u16,
    },
    PagingUnavailable,
    PageNotMapped {
        addr: u64,
    },
    PageFlagUpdate {
        addr: u64,
    },
    UnsupportedComplexElf {
        entry: u64,
        segments: usize,
        memory_size: usize,
    },
}

const MAX_NATIVE_MEM: usize = 32 * 1024 * 1024;
const STACK_SIZE: usize = 256 * 1024;
const AT_NULL: u64 = 0;
const AT_PAGESZ: u64 = 6;

#[no_mangle]
static mut leonos3_native_return_rsp: u64 = 0;
#[no_mangle]
static mut leonos3_native_return_rip: u64 = 0;

static mut SYSCALL_READY: bool = false;
static mut OUT_CTX: *mut () = core::ptr::null_mut();
static mut OUT_FN: Option<unsafe fn(*mut (), *const u8, usize)> = None;
static mut PHYSICAL_MEMORY_OFFSET: Option<u64> = None;

global_asm!(
    r#"
.intel_syntax noprefix

.global leonos3_native_enter
leonos3_native_enter:
    mov [rip + leonos3_native_return_rsp], rsp
    lea rax, [rip + .Lnative_return]
    mov [rip + leonos3_native_return_rip], rax
    mov rsp, rsi
    call rdi
.Lnative_return:
    ret

.global leonos3_native_syscall_entry
leonos3_native_syscall_entry:
    cmp rax, 60
    je .Lnative_exit
    cmp rax, 231
    je .Lnative_exit

    push rcx
    push r11
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15

    mov rbx, rdi
    mov rbp, rsi
    mov r12, rdx
    mov r13, r10
    mov r14, r8
    mov r15, r9

    mov rdi, rax
    mov rsi, rbx
    mov rdx, rbp
    mov rcx, r12
    mov r8,  r13
    mov r9,  r14
    call leonos3_native_syscall_dispatch

    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    pop r11
    pop rcx
    jmp rcx

.Lnative_exit:
    call leonos3_native_mark_exit
    mov rsp, [rip + leonos3_native_return_rsp]
    jmp qword ptr [rip + leonos3_native_return_rip]
"#
);

unsafe extern "C" {
    fn leonos3_native_enter(entry: usize, user_rsp: usize);
    fn leonos3_native_syscall_entry();
}

#[no_mangle]
extern "C" fn leonos3_native_mark_exit(code: u64) {
    syscall::mark_exit_code(code as i32);
}

#[no_mangle]
extern "C" fn leonos3_native_syscall_dispatch(
    nr: u64,
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
) -> u64 {
    match nr as usize {
        syscall::SYS_WRITE => {
            let fd = a0 as usize;
            let ptr = a1 as *const u8;
            let len = a2 as usize;
            if (fd == 1 || fd == 2) && !ptr.is_null() && len != 0 {
                unsafe {
                    if let Some(writer) = OUT_FN {
                        writer(OUT_CTX, ptr, len);
                    }
                }
                return len as u64;
            }
            syscall::dispatch(nr as usize, a0 as usize, a1 as usize, a2 as usize, a3 as usize, a4 as usize, 0)
                as u64
        }
        syscall::SYS_READ => {
            let fd = a0 as usize;
            if fd == 0 {
                let ptr = a1 as *mut u8;
                let len = a2 as usize;
                if ptr.is_null() || len == 0 {
                    return 0;
                }
                let mut got = 0usize;
                while got < len {
                    let Some(ch) = syscall::read_stdin_byte_blocking() else {
                        break;
                    };
                    let mapped = if ch == b'\r' { b'\n' } else { ch };
                    unsafe {
                        ptr.add(got).write(mapped);
                    }
                    got += 1;
                    if mapped == b'\n' {
                        break;
                    }
                }
                return got as u64;
            }
            syscall::dispatch(nr as usize, a0 as usize, a1 as usize, a2 as usize, a3 as usize, a4 as usize, 0)
                as u64
        }
        _ => syscall::dispatch(nr as usize, a0 as usize, a1 as usize, a2 as usize, a3 as usize, a4 as usize, 0) as u64,
    }
}

pub fn run_native_elf_with_args<D: DrawTarget>(
    terminal: &mut Terminal<D>,
    image: &[u8],
    argv: &[&str],
    envp: &[&str],
) -> Result<(), NativeError> {
    serial_write("[LeonOS3] native step: enter runner\n");
    serial_write("[LeonOS3] native step: init syscall path begin\n");
    init_syscall_path();
    serial_write("[LeonOS3] native step: init syscall path done\n");

    serial_write("[LeonOS3] native step: parse elf begin\n");
    let elf = elf::parse_elf64(image).map_err(NativeError::Parse)?;
    serial_write("[LeonOS3] native step: parse elf done\n");
    if elf.segments.is_empty() {
        serial_write("[LeonOS3] native step: no load segment\n");
        return Err(NativeError::NoLoadSegment);
    }

    serial_write("[LeonOS3] native step: scan segments begin\n");
    let mut min_vaddr = u64::MAX;
    let mut max_vaddr = 0u64;
    for seg in &elf.segments {
        min_vaddr = min_vaddr.min(seg.vaddr);
        max_vaddr = max_vaddr.max(seg.vaddr.saturating_add(seg.mem_size));
    }
    serial_write("[LeonOS3] native step: scan segments done\n");

    if elf.e_type == 2 {
        serial_write("[LeonOS3] native step: ET_EXEC fixed-vaddr not supported in relocate mode\n");
        return Err(NativeError::FixedVaddrUnsupported {
            min_vaddr,
            e_type: elf.e_type,
        });
    }

    if max_vaddr <= min_vaddr {
        serial_write("[LeonOS3] native step: invalid vaddr range\n");
        return Err(NativeError::AddressOutOfRange);
    }

    let mem_len = (max_vaddr - min_vaddr) as usize;
    serial_write(&format!(
        "[LeonOS3] native size: segs={} image={} mem={}\n",
        elf.segments.len(),
        image.len(),
        mem_len
    ));
    if mem_len > MAX_NATIVE_MEM || elf.segments.len() > 16 {
        serial_write("[LeonOS3] native step: elf too complex for runner\n");
        return Err(NativeError::UnsupportedComplexElf {
            entry: elf.entry,
            segments: elf.segments.len(),
            memory_size: mem_len,
        });
    }

    let total_len = mem_len
        .checked_add(STACK_SIZE)
        .ok_or(NativeError::AddressOutOfRange)?;
    serial_write("[LeonOS3] native step: alloc image memory begin\n");
    let mut mem = vec![0u8; total_len];
    serial_write("[LeonOS3] native step: alloc image memory done\n");

    serial_write("[LeonOS3] native step: copy segments begin\n");
    for (idx, seg) in elf.segments.iter().enumerate() {
        serial_write(&format!(
            "[LeonOS3] native seg[{}]: vaddr=0x{:x} mem=0x{:x} file_off=0x{:x} file_sz=0x{:x}\n",
            idx, seg.vaddr, seg.mem_size, seg.file_offset, seg.file_size
        ));

        let dst_start = (seg.vaddr - min_vaddr) as usize;
        let dst_end = dst_start.saturating_add(seg.mem_size as usize);
        if dst_end > mem.len() {
            serial_write("[LeonOS3] native step: copy segments dst out of range\n");
            return Err(NativeError::AddressOutOfRange);
        }

        let src_start = seg.file_offset as usize;
        let src_end = src_start.saturating_add(seg.file_size as usize);
        if src_end > image.len() {
            serial_write("[LeonOS3] native step: copy segments src out of range\n");
            return Err(NativeError::AddressOutOfRange);
        }

        let file_len = seg.file_size as usize;
        mem[dst_start..dst_start + file_len].copy_from_slice(&image[src_start..src_end]);
    }
    serial_write("[LeonOS3] native step: copy segments done\n");

    let runtime_base = mem.as_ptr() as u64;
    let stack_top = runtime_base + (mem.len() as u64) - 0x10;
    serial_write("[LeonOS3] native step: build user stack begin\n");
    let user_rsp = build_initial_stack(runtime_base, &mut mem, stack_top, argv, envp)?;
    serial_write("[LeonOS3] native step: build user stack done\n");

    let entry_off = elf
        .entry
        .checked_sub(min_vaddr)
        .ok_or(NativeError::AddressOutOfRange)?;
    let mut entry = runtime_base + entry_off;

    let entry_idx = va_to_index(runtime_base, &mem, entry)?;
    let preview_len = core::cmp::min(16usize, mem.len().saturating_sub(entry_idx));
    let preview = &mem[entry_idx..entry_idx + preview_len];
    serial_write(&format!(
        "[LeonOS3] native entry bytes: {}\n",
        hex_bytes(preview)
    ));
    if preview.starts_with(&[0xF3, 0x0F, 0x1E, 0xFA]) {
        serial_write("[LeonOS3] native step: entry starts with ENDBR64, skip +4\n");
        entry = entry.saturating_add(4);
    }

    serial_write("[LeonOS3] native step: make native pages executable begin\n");
    ensure_region_executable(runtime_base, mem.len())?;
    serial_write("[LeonOS3] native step: make native pages executable done\n");

    serial_write(&format!(
        "[LeonOS3] native map base=0x{:x} entry=0x{:x} rsp=0x{:x}\n",
        runtime_base, entry, user_rsp
    ));

    syscall::mark_exit_code(0);
    unsafe {
        serial_write("[LeonOS3] native step: install terminal writer\n");
        OUT_CTX = (terminal as *mut Terminal<D>).cast::<()>();
        OUT_FN = Some(term_write_impl::<D>);
        serial_write("[LeonOS3] native step: jump to user entry\n");
        leonos3_native_enter(entry as usize, user_rsp as usize);
        serial_write("[LeonOS3] native returned to kernel\n");
        OUT_FN = None;
        OUT_CTX = core::ptr::null_mut();
        serial_write("[LeonOS3] native step: clear terminal writer\n");
    }

    Ok(())
}

pub fn set_physical_memory_offset(offset: u64) {
    unsafe {
        PHYSICAL_MEMORY_OFFSET = Some(offset);
    }
}

fn init_syscall_path() {
    unsafe {
        if SYSCALL_READY {
            serial_write("[LeonOS3] native syscall path: already ready\n");
            return;
        }

        serial_write("[LeonOS3] native syscall path: set EFER begin\n");
        Efer::update(|efer| {
            *efer |= EferFlags::SYSTEM_CALL_EXTENSIONS;
            
        });
        serial_write("[LeonOS3] native syscall path: set EFER done (NXE unchanged)\n");

        const KERNEL_CS: u16 = 0x08;
        serial_write("[LeonOS3] native syscall path: use fixed cs=0x08\n");

        serial_write("[LeonOS3] native syscall path: write STAR begin\n");
        Star::write_raw(0x13, KERNEL_CS);
        serial_write("[LeonOS3] native syscall path: write STAR done\n");

        serial_write("[LeonOS3] native syscall path: write LSTAR begin\n");
        LStar::write(VirtAddr::new(leonos3_native_syscall_entry as usize as u64));
        serial_write("[LeonOS3] native syscall path: write LSTAR done\n");

        serial_write("[LeonOS3] native syscall path: write SFMASK begin\n");
        SFMask::write(RFlags::INTERRUPT_FLAG);
        serial_write("[LeonOS3] native syscall path: write SFMASK done\n");

        SYSCALL_READY = true;
        serial_write("[LeonOS3] native syscall path: ready\n");
    }
}

fn ensure_region_executable(start: u64, len: usize) -> Result<(), NativeError> {
    if len == 0 {
        return Ok(());
    }

    let phys_off = unsafe { PHYSICAL_MEMORY_OFFSET }.ok_or(NativeError::PagingUnavailable)?;
    let phys_off_virt = VirtAddr::new(phys_off);

    let start_addr = VirtAddr::new(start);
    let end_addr = VirtAddr::new(
        start
            .checked_add(len as u64)
            .and_then(|v| v.checked_sub(1))
            .ok_or(NativeError::AddressOutOfRange)?,
    );

    let start_page = Page::<Size4KiB>::containing_address(start_addr);
    let end_page = Page::<Size4KiB>::containing_address(end_addr);

    unsafe {
        let level_4 = active_level_4_table(phys_off_virt);
        let mut mapper = OffsetPageTable::new(level_4, phys_off_virt);

        for page in Page::range_inclusive(start_page, end_page) {
            let vaddr = page.start_address();
            let flags = match mapper.translate(vaddr) {
                TranslateResult::Mapped { flags, .. } => flags,
                _ => {
                    return Err(NativeError::PageNotMapped {
                        addr: vaddr.as_u64(),
                    });
                }
            };

            if flags.contains(PageTableFlags::NO_EXECUTE) {
                let new_flags = flags & !PageTableFlags::NO_EXECUTE;
                match mapper.update_flags(page, new_flags) {
                    Ok(flush) => flush.flush(),
                    Err(_) => {
                        return Err(NativeError::PageFlagUpdate {
                            addr: vaddr.as_u64(),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_frame, _) = Cr3::read();
    let phys = level_4_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}

unsafe fn term_write_impl<D: DrawTarget>(ctx: *mut (), ptr: *const u8, len: usize) {
    if ctx.is_null() || ptr.is_null() || len == 0 {
        return;
    }
    let terminal = &mut *(ctx.cast::<Terminal<D>>());
    let bytes = core::slice::from_raw_parts(ptr, len);
    terminal.process(bytes);
}

fn build_initial_stack(
    base: u64,
    mem: &mut [u8],
    stack_top: u64,
    argv: &[&str],
    envp: &[&str],
) -> Result<u64, NativeError> {
    let mut sp = stack_top & !0xf;

    let mut arg_ptrs = Vec::new();
    for i in (0..argv.len()).rev() {
        sp = push_cstr(base, mem, sp, argv[i].as_bytes())?;
        arg_ptrs.push(sp);
    }
    arg_ptrs.reverse();

    let mut env_ptrs = Vec::new();
    for i in (0..envp.len()).rev() {
        sp = push_cstr(base, mem, sp, envp[i].as_bytes())?;
        env_ptrs.push(sp);
    }
    env_ptrs.reverse();

    let mut words = Vec::new();
    words.push(arg_ptrs.len() as u64);
    for p in &arg_ptrs {
        words.push(*p);
    }
    words.push(0);
    for p in &env_ptrs {
        words.push(*p);
    }
    words.push(0);
    words.push(AT_PAGESZ);
    words.push(4096);
    words.push(AT_NULL);
    words.push(0);

    sp &= !0xf;
    let bytes = (words.len() * 8) as u64;
    sp = sp
        .checked_sub(bytes)
        .ok_or(NativeError::AddressOutOfRange)?;
    for (i, v) in words.iter().enumerate() {
        write_u64(base, mem, sp + (i as u64) * 8, *v)?;
    }

    Ok(sp)
}

fn push_cstr(base: u64, mem: &mut [u8], sp: u64, bytes: &[u8]) -> Result<u64, NativeError> {
    let len = bytes.len() + 1;
    let new_sp = sp
        .checked_sub(len as u64)
        .ok_or(NativeError::AddressOutOfRange)?;
    let start = va_to_index(base, mem, new_sp)?;
    let end = start.checked_add(len).ok_or(NativeError::AddressOutOfRange)?;
    if end > mem.len() {
        return Err(NativeError::AddressOutOfRange);
    }
    mem[start..start + bytes.len()].copy_from_slice(bytes);
    mem[start + bytes.len()] = 0;
    Ok(new_sp)
}

fn va_to_index(base: u64, mem: &[u8], va: u64) -> Result<usize, NativeError> {
    if va < base {
        return Err(NativeError::AddressOutOfRange);
    }
    let idx = (va - base) as usize;
    if idx >= mem.len() {
        return Err(NativeError::AddressOutOfRange);
    }
    Ok(idx)
}

fn write_u64(base: u64, mem: &mut [u8], va: u64, v: u64) -> Result<(), NativeError> {
    let i = va_to_index(base, mem, va)?;
    let end = i.checked_add(8).ok_or(NativeError::AddressOutOfRange)?;
    if end > mem.len() {
        return Err(NativeError::AddressOutOfRange);
    }
    mem[i..end].copy_from_slice(&v.to_le_bytes());
    Ok(())
}





fn hex_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (i, b) in bytes.iter().enumerate() {
        let _ = write!(out, "{:02x}", b);
        if i + 1 != bytes.len() {
            out.push(' ');
        }
    }
    out
}












