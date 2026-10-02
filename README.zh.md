# OpenShark-R1

适用于 Linux 的 **Attack Shark R1** 鼠标驱动与托盘工具。

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-green)](LICENSE)
![Platform: Linux](https://img.shields.io/badge/platform-Linux-lightgrey)

[English](README.md) | [Русский](README.ru.md) | **中文** | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

托盘中有两个图标：鼠标图标和电量（每分钟刷新一次）。设置窗口在启动时打开；关闭后会隐藏到托盘，通过托盘菜单的 **“设置…”** 可以再次打开。

<details>
<summary><b>截图</b></summary>

![OpenShark R1 设置窗口](static/screenshot.png)

</details>

## 功能

- 托盘中的电池电量（2.4G 接收器），按电量变色
- 设置窗口：回报率、6 档 DPI 及当前档位、休眠定时、按键响应、ripple control / angle snap
- **按键恢复**：写回出厂按键映射表（当按键被映射到空操作导致“点击无效”时可修复）
- 设置保存在 `~/.config/openshark-r1/config.json`
- 普通用户即可运行（通过 udev 规则）
- Rust 后端基于 libusb（`rusb`），界面使用 Tauri 2 + Svelte

## 安装

从源码构建软件包（依赖见 [开发](#开发)）：

```sh
pnpm install
pnpm tauri build
```

构建产物位于 `target/release/bundle/`：deb、rpm、AppImage。

以 root 安装一次 udev 规则，否则只有 root 能访问鼠标：

```sh
sudo cp udev/99-attack-shark-r1.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --action=change --subsystem-match=usb
```

## 开发

依赖（Arch/CachyOS）：

```sh
sudo pacman -S --needed webkit2gtk-4.1 libayatana-appindicator librsvg \
    gtk3 pkgconf base-devel libusb rustup
```

udev 规则安装一次，见 [安装](#安装)。

开发模式运行：

```sh
pnpm install
pnpm tauri dev
```

发布构建：

```sh
pnpm tauri build
```

### AppImage（Arch/CachyOS）

Tauri 为 AppImage 下载的缓存在新系统上会出问题：旧版 linuxdeploy 的
`strip` 在 `.relr.dyn` 上失败，gtk 插件又强制 `GDK_BACKEND=x11`
（在 NVIDIA + Wayland 上窗口会变成空白）。一次性准备缓存：

```sh
sudo pacman -S --needed patchelf
pnpm tauri build || true   # 填充缓存；此处 strip 可能失败，属于正常
bash scripts/appimage-fix.sh
pnpm tauri build
```

清理 `~/.cache/tauri` 后需要重新执行带 `appimage-fix.sh` 的步骤。

### 测试

驱动测试（硬件测试默认关闭）：

```sh
cargo test                # 仅单元测试，不触碰鼠标
cargo test -- --ignored   # 另加真实设备上的测试
```

从终端查看电量：

```sh
cargo run -p openshark-driver --example battery
```

从终端把按键恢复为出厂功能：

```sh
cargo run -p openshark-driver --example buttons
```

## 项目结构

| 路径                            | 用途                                 |
| ------------------------------- | ------------------------------------ |
| `crates/openshark-driver`       | 底层 USB 驱动（libusb），无 GUI      |
| `src-tauri`                     | Tauri 后端：托盘、命令、电量轮询     |
| `src/` + `static/`              | SvelteKit 设置窗口界面              |
| `udev/99-attack-shark-r1.rules` | 普通用户的设备访问权限              |

## 协议

设备：VID `0x1d57`，PID `0xfa60`（2.4G）/ `0xfa61`（有线），interface 2。

- 电池：interrupt-IN EP `0x83`，报告第 `4` 字节 × 10 = 百分比
- 配置（DPI、回报率、定时器）：control transfer `SET_REPORT`
  （`0x21/0x09`，wValue `0x304`–`0x306`），ACK 从同一 EP `0x83` 返回，
  条件为 `buf[2] == 0x50`
- 按键表：feature report `0x08`，59 字节：`08 3b 01` + 18 个 3 字节槽位 +
  校验和（最后一个字节为 `bytes[2..58]` 之和减 1）

协议实现参考了 Odin 版本：
[xb-bx/attack-shark-r1-driver](https://github.com/xb-bx/attack-shark-r1-driver)。
按键表格式来自 Attack Shark 协议家族（report `0x08`）。

> **注意：** 鼠标必须先被唤醒（晃动它），否则报告不会被 ACK，表现为
> “鼠标无响应”。

## 说明

- **NVIDIA + Wayland：** 显示窗口时 WebKitGTK 会报 `Error 71 (Protocol error)`。
  `src-tauri/src/main.rs` 检测到 NVIDIA 驱动时会自动设置
  `__NV_DISABLE_EXPLICIT_SYNC=1`，无需手动配置。
- 窗口高于可见区域（内容约 1100px）：“应用”按钮和状态栏固定在底部，
  主体部分可滚动。
- 不要在玩游戏时运行硬件测试（`--ignored`）：`apply_config` 会改写鼠标设置。

## 状态

- [x] 读取电量，托盘指示
- [x] 设置窗口（DPI、回报率、定时器），保存为 JSON
- [x] 恢复出厂按键表

**许可证：** [GPL-3.0](LICENSE)。
