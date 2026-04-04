#[derive(Clone, Copy, Debug)]
pub enum FsError {
    TooSmall,
    BadMagic,
    BadVersion,
    OutOfBounds,
    InvalidName,
    NotFound,
}

#[derive(Clone, Copy)]
pub struct FsHeader {
    pub file_count: u16,
}

#[derive(Clone, Copy)]
pub struct FileRecord<'a> {
    pub name: &'a str,
    pub mode: u16,
    pub data: &'a [u8],
}

const MAGIC: &[u8; 4] = b"LFS1";
const VERSION: u16 = 1;
const HEADER_SIZE: usize = 12;
const ENTRY_SIZE: usize = 64;
const NAME_CAP: usize = 48;

pub fn header(image: &[u8]) -> Result<FsHeader, FsError> {
    if image.len() < HEADER_SIZE {
        return Err(FsError::TooSmall);
    }
    if &image[0..4] != MAGIC {
        return Err(FsError::BadMagic);
    }

    let version = u16::from_le_bytes([image[4], image[5]]);
    if version != VERSION {
        return Err(FsError::BadVersion);
    }

    let file_count = u16::from_le_bytes([image[6], image[7]]);
    let table_len = (file_count as usize)
        .checked_mul(ENTRY_SIZE)
        .ok_or(FsError::OutOfBounds)?;
    let table_end = HEADER_SIZE.checked_add(table_len).ok_or(FsError::OutOfBounds)?;
    if table_end > image.len() {
        return Err(FsError::OutOfBounds);
    }

    Ok(FsHeader { file_count })
}

pub fn list(image: &[u8], mut f: impl FnMut(FileRecord<'_>)) -> Result<(), FsError> {
    let hdr = header(image)?;
    for i in 0..(hdr.file_count as usize) {
        let rec = entry_at(image, i)?;
        f(rec);
    }
    Ok(())
}

pub fn open<'a>(image: &'a [u8], wanted: &str) -> Result<FileRecord<'a>, FsError> {
    let hdr = header(image)?;
    for i in 0..(hdr.file_count as usize) {
        let rec = entry_at(image, i)?;
        if rec.name == wanted {
            return Ok(rec);
        }
    }
    Err(FsError::NotFound)
}

pub fn entry<'a>(image: &'a [u8], idx: usize) -> Result<FileRecord<'a>, FsError> {
    let hdr = header(image)?;
    if idx >= hdr.file_count as usize {
        return Err(FsError::NotFound);
    }
    entry_at(image, idx)
}

fn entry_at<'a>(image: &'a [u8], idx: usize) -> Result<FileRecord<'a>, FsError> {
    let start = HEADER_SIZE
        .checked_add(idx.checked_mul(ENTRY_SIZE).ok_or(FsError::OutOfBounds)?)
        .ok_or(FsError::OutOfBounds)?;
    let end = start.checked_add(ENTRY_SIZE).ok_or(FsError::OutOfBounds)?;
    if end > image.len() {
        return Err(FsError::OutOfBounds);
    }

    let e = &image[start..end];
    let name_len = u16::from_le_bytes([e[0], e[1]]) as usize;
    if name_len == 0 || name_len > NAME_CAP {
        return Err(FsError::InvalidName);
    }

    let offset = u32::from_le_bytes([e[4], e[5], e[6], e[7]]) as usize;
    let size = u32::from_le_bytes([e[8], e[9], e[10], e[11]]) as usize;
    let mode = u16::from_le_bytes([e[12], e[13]]);

    let name_bytes = &e[16..(16 + name_len)];
    let name = core::str::from_utf8(name_bytes).map_err(|_| FsError::InvalidName)?;

    let data_end = offset.checked_add(size).ok_or(FsError::OutOfBounds)?;
    if data_end > image.len() {
        return Err(FsError::OutOfBounds);
    }

    Ok(FileRecord {
        name,
        mode,
        data: &image[offset..data_end],
    })
}

