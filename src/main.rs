use std::process::Command;

fn main() {
    let bios_path = env!("BIOS_PATH");

    let status = Command::new("qemu-system-x86_64")
        .args([
            "-drive",
            &format!("format=raw,file={bios_path}"),
            "-vga",
            "std",
            "-serial",
            "stdio",
            "-monitor",
            "none",
        ])
        .status()
        .expect("failed to start qemu-system-x86_64");

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
