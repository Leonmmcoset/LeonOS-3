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
    println!("cargo:rerun-if-changed=userspace/exit7");
    println!("cargo:rerun-if-changed=userspace/wait4_echild");
    println!("cargo:rerun-if-changed=userspace/wait4_reap");
    println!("cargo:rerun-if-changed=userspace/sbase-box");
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

    maybe_add_file(&mut files, "hello_world", "userspace/hello_world", 0o100755);
    maybe_add_file(&mut files, "c_hello_name", "userspace/c_hello_name", 0o100755);
    maybe_add_file(&mut files, "exit7", "userspace/exit7", 0o100755);
    maybe_add_file(&mut files, "wait4_echild", "userspace/wait4_echild", 0o100755);
    maybe_add_file(&mut files, "wait4_reap", "userspace/wait4_reap", 0o100755);

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
        println!("cargo:warning=userspace/busybox not found; place binary at userspace/busybox");
    }

    let sbase_path = Path::new("userspace/sbase-box");
    if sbase_path.exists() {
        let bin = fs::read(sbase_path).expect("failed to read userspace/sbase-box");
        files.push(BuildFile {
            name: "sbase",
            mode: 0o100755,
            data: bin,
        });
    } else {
        println!("cargo:warning=userspace/sbase-box not found; place binary at userspace/sbase-box if needed");
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

fn maybe_add_file(files: &mut Vec<BuildFile>, name: &'static str, path: &str, mode: u16) {
    let p = Path::new(path);
    if p.exists() {
        let data = fs::read(p).unwrap_or_else(|_| panic!("failed to read {}", path));
        files.push(BuildFile { name, mode, data });
    } else {
        println!("cargo:warning={} not found; run `make userspace`", path);
    }
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
