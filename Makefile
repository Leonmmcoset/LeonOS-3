SHELL := /bin/bash

CARGO := cargo +nightly

.PHONY: all help setup check userspace build run clean

all: run

help:
	@echo "LeonOS 3 Make targets:"
	@echo "  make          - One-shot: userspace + check + run"
	@echo "  make setup    - Install nightly toolchain components"
	@echo "  make check    - Check toolchain, target and qemu"
	@echo "  make userspace- Build userspace hello_world ELF"
	@echo "  make build    - Build runner and kernel image artifacts"
	@echo "  make run      - Build and run in QEMU"
	@echo "  make clean    - Clean target directory"

setup:
	rustup toolchain install nightly
	rustup target add x86_64-unknown-none --toolchain nightly
	rustup component add rust-src --toolchain nightly
	rustup component add llvm-tools-preview --toolchain nightly

check:
	@rustup toolchain list | grep -q nightly || (echo "Error: nightly toolchain not installed" && exit 1)
	@rustup target list --installed --toolchain nightly | grep -q '^x86_64-unknown-none$$' || (echo "Error: x86_64-unknown-none target missing (run: make setup)" && exit 1)
	@command -v qemu-system-x86_64 >/dev/null 2>&1 || (echo "Error: qemu-system-x86_64 not found in PATH" && exit 1)

userspace:
	$(MAKE) -C userspace all

build: userspace check
	$(CARGO) build -Z bindeps

run: userspace check
	$(CARGO) run -Z bindeps

clean:
	cargo clean
	$(MAKE) -C userspace clean
