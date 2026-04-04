use std::fs;
use std::path::{Path, PathBuf};

const ENTRY_SIZE: usize = 64;
const NAME_CAP: usize = 48;

struct BuildFile {
    name: &'static str,
    mode: u16,
    data: Vec<u8>,
}

fn main() {
    println!("cargo:rerun-if-changed=ramdisk/initrd.txt");
    println!("cargo:rerun-if-changed=userspace/hello_world");
    println!("cargo:rerun-if-changed=userspace/c_hello_name");
    println!("cargo:rerun-if-changed=userspace/busybox");

    let kernel = PathBuf::from(
        std::env::var_os("CARGO_BIN_FILE_KERNEL_kernel")
            .expect("kernel binary path not provided by Cargo"),
    );

    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR not set");
    let out_dir = PathBuf::from(out_dir);
    let bios_path = out_dir.join("leonos3-bios.img");
    let ramdisk_path = out_dir.join("leonos3.ramdisk");

    let mut files = Vec::new();
    let initrd = fs::read("ramdisk/initrd.txt").unwrap_or_else(|_| b"LeonOS 3 ramdisk\n".to_vec());
    files.push(BuildFile {
        name: "initrd.txt",
        mode: 0o100644,
        data: initrd,
    });

    let user_elf_path = Path::new("userspace/hello_world");
    if user_elf_path.exists() {
        let elf = fs::read(user_elf_path).expect("failed to read userspace/hello_world");
        files.push(BuildFile {
            name: "hello_world",
            mode: 0o100755,
            data: elf,
        });
    } else {
        println!("cargo:warning=userspace/hello_world not found; run `make userspace`");
    }

    let c_hello_path = Path::new("userspace/c_hello_name");
    if c_hello_path.exists() {
        let elf = fs::read(c_hello_path).expect("failed to read userspace/c_hello_name");
        files.push(BuildFile {
            name: "c_hello_name",
            mode: 0o100755,
            data: elf,
        });
    } else {
        println!("cargo:warning=userspace/c_hello_name not found; run `make userspace`");
    }

    let busybox_path = Path::new("userspace/busybox");
    if busybox_path.exists() {
        let bin = fs::read(busybox_path).expect("failed to read userspace/busybox");
        files.push(BuildFile {
            name: "busybox",
            mode: 0o100755,
            data: bin.clone(),
        });
        files.push(BuildFile {
            name: "sh",
            mode: 0o100755,
            data: bin,
        });
    } else {
        println!("cargo:warning=userspace/busybox not found; place busybox binary at userspace/busybox");
    }

    let ramdisk = build_lfs1(&files);
    fs::write(&ramdisk_path, ramdisk).expect("failed to write generated ramdisk image");

    let mut boot = bootloader::BiosBoot::new(&kernel);
    let mut config = bootloader::BootConfig::default();
    config.frame_buffer.minimum_framebuffer_width = Some(1024);
    config.frame_buffer.minimum_framebuffer_height = Some(768);

    boot.set_boot_config(&config)
        .set_ramdisk(&ramdisk_path)
        .create_disk_image(&bios_path)
        .expect("failed to create BIOS disk image");

    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
}

fn build_lfs1(files: &[BuildFile]) -> Vec<u8> {
    let header_size = 12usize;
    let table_size = files.len() * ENTRY_SIZE;
    let mut blob = vec![0u8; header_size + table_size];

    blob[0..4].copy_from_slice(b"LFS1");
    blob[4..6].copy_from_slice(&1u16.to_le_bytes());
    blob[6..8].copy_from_slice(&(files.len() as u16).to_le_bytes());

    let mut cursor = header_size + table_size;

    for (i, file) in files.iter().enumerate() {
        assert!(file.name.len() <= NAME_CAP, "file name too long for LFS1");
        let entry_off = header_size + i * ENTRY_SIZE;

        let name_len = file.name.len() as u16;
        let offset = cursor as u32;
        let size = file.data.len() as u32;

        blob[entry_off..entry_off + 2].copy_from_slice(&name_len.to_le_bytes());
        blob[entry_off + 4..entry_off + 8].copy_from_slice(&offset.to_le_bytes());
        blob[entry_off + 8..entry_off + 12].copy_from_slice(&size.to_le_bytes());
        blob[entry_off + 12..entry_off + 14].copy_from_slice(&file.mode.to_le_bytes());

        let name_bytes = file.name.as_bytes();
        blob[entry_off + 16..entry_off + 16 + name_bytes.len()].copy_from_slice(name_bytes);

        blob.extend_from_slice(&file.data);
        cursor += file.data.len();
    }

    blob
}
