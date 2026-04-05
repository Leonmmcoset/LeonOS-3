extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug)]
pub struct LoadSegment {
    pub vaddr: u64,
    pub mem_size: u64,
    pub file_offset: u64,
    pub file_size: u64,
    pub flags: u32,
}

#[derive(Debug)]
pub struct ElfImage {
    pub entry: u64,
    pub e_type: u16,
    pub segments: Vec<LoadSegment>,
}

#[derive(Clone, Copy, Debug)]
pub enum ElfError {
    TooSmall,
    BadMagic,
    UnsupportedClass,
    UnsupportedEndian,
    UnsupportedMachine,
    BadHeader,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EM_X86_64: u16 = 62;
const PT_LOAD: u32 = 1;

pub fn parse_elf64(image: &[u8]) -> Result<ElfImage, ElfError> {
    if image.len() < core::mem::size_of::<Elf64Ehdr>() {
        return Err(ElfError::TooSmall);
    }

    let ehdr = read_unaligned::<Elf64Ehdr>(image.as_ptr());

    if ehdr.e_ident[0..4] != [0x7F, b'E', b'L', b'F'] {
        return Err(ElfError::BadMagic);
    }
    if ehdr.e_ident[EI_CLASS] != ELFCLASS64 {
        return Err(ElfError::UnsupportedClass);
    }
    if ehdr.e_ident[EI_DATA] != ELFDATA2LSB {
        return Err(ElfError::UnsupportedEndian);
    }
    if ehdr.e_machine != EM_X86_64 {
        return Err(ElfError::UnsupportedMachine);
    }
    if ehdr.e_phentsize as usize != core::mem::size_of::<Elf64Phdr>() {
        return Err(ElfError::BadHeader);
    }

    let phoff = ehdr.e_phoff as usize;
    let phnum = ehdr.e_phnum as usize;
    let phentsize = ehdr.e_phentsize as usize;

    let table_len = phnum.checked_mul(phentsize).ok_or(ElfError::BadHeader)?;
    let end = phoff.checked_add(table_len).ok_or(ElfError::BadHeader)?;
    if end > image.len() {
        return Err(ElfError::BadHeader);
    }

    let mut segments = Vec::new();
    for i in 0..phnum {
        let off = phoff + i * phentsize;
        let ph = read_unaligned::<Elf64Phdr>(unsafe { image.as_ptr().add(off) });
        if ph.p_type == PT_LOAD {
            segments.push(LoadSegment {
                vaddr: ph.p_vaddr,
                mem_size: ph.p_memsz,
                file_offset: ph.p_offset,
                file_size: ph.p_filesz,
                flags: ph.p_flags,
            });
        }
    }

    Ok(ElfImage {
        entry: ehdr.e_entry,
        e_type: ehdr.e_type,
        segments,
    })
}

fn read_unaligned<T: Copy>(ptr: *const u8) -> T {
    unsafe { core::ptr::read_unaligned(ptr as *const T) }
}
