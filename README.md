# LeonOS 3

一个使用 Rust 编写的最小操作系统内核示例，当前已实现 `OSTerminal` 文本输出并打印 `HelloWorld`。

## 当前能力

- 裸机内核入口（`no_std` / `no_main`）
- `OSTerminal` VGA 文本终端输出
- 启动后输出：
  - `LeonOS 3 booting...`
  - `HelloWorld`

## 运行步骤（Windows）

1. 安装 nightly 和 bootimage 工具：

```powershell
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
cargo install bootimage
rustup component add llvm-tools-preview --toolchain nightly
```

2. 构建并运行：

```powershell
cargo +nightly bootimage
cargo +nightly run
```

> 需要本机安装 QEMU（例如 `qemu-system-x86_64` 在 PATH 中）。
