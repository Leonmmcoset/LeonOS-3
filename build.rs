fn main() {
    let kernel = std::path::PathBuf::from(
        std::env::var_os("CARGO_BIN_FILE_KERNEL_kernel")
            .expect("kernel binary path not provided by Cargo"),
    );

    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR not set");
    let bios_path = std::path::Path::new(&out_dir).join("leonos3-bios.img");

    let mut boot = bootloader::BiosBoot::new(&kernel);
    let mut config = bootloader::BootConfig::default();
    config.frame_buffer.minimum_framebuffer_width = Some(1024);
    config.frame_buffer.minimum_framebuffer_height = Some(768);
    boot.set_boot_config(&config)
        .create_disk_image(&bios_path)
        .expect("failed to create BIOS disk image");

    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
}
