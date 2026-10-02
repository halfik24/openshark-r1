# OpenShark-R1

Driver and tray utility for the **Attack Shark R1** mouse on Linux.

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-green)](LICENSE)
![Platform: Linux](https://img.shields.io/badge/platform-Linux-lightgrey)

**English** | [Русский](README.ru.md) | [中文](README.zh.md) | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

The tray holds two icons: the mouse icon and the battery level (updated once a minute). The settings window opens at start; closing it hides the window to the tray, and the **Settings...** tray item opens it again.

<details>
<summary><b>Screenshot</b></summary>

![OpenShark R1 settings window](static/screenshot.png)

</details>

## Features

- Battery level in the tray (2.4G receiver), color coded by charge
- Settings window: polling rate, 6 DPI steps and the active step, sleep timers, click response, ripple control / angle snap
- **Button restore**: writes back the factory remap table (fixes "clicks do nothing" when buttons are mapped to nothing)
- Settings stored in `~/.config/openshark-r1/config.json`
- Runs as a regular user (through a udev rule)
- Rust backend on top of libusb (`rusb`), GUI on Tauri 2 + Svelte

## Installation

Build the packages from source (dependencies are listed under [Development](#development)):

```sh
pnpm install
pnpm tauri build
```

The artifacts land in `target/release/bundle/`: deb, rpm, AppImage.

Install the udev rule once from root, otherwise the mouse is reachable by root only:

```sh
sudo cp udev/99-attack-shark-r1.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --action=change --subsystem-match=usb
```

## Development

Dependencies (Arch/CachyOS):

```sh
sudo pacman -S --needed webkit2gtk-4.1 libayatana-appindicator librsvg \
    gtk3 pkgconf base-devel libusb rustup
```

The udev rule is installed once, see [Installation](#installation).

Run in development mode:

```sh
pnpm install
pnpm tauri dev
```

Release build:

```sh
pnpm tauri build
```

### AppImage (Arch/CachyOS)

The cache Tauri downloads for AppImage breaks on fresh systems: the old
linuxdeploy build fails with its `strip` on `.relr.dyn`, and the gtk plugin
forces `GDK_BACKEND=x11`, which leaves the window blank on NVIDIA + Wayland.
Prepare the cache once:

```sh
sudo pacman -S --needed patchelf
pnpm tauri build || true   # fills the cache; strip may fail here, that is fine
bash scripts/appimage-fix.sh
pnpm tauri build
```

After clearing `~/.cache/tauri`, repeat the `appimage-fix.sh` steps.

### Tests

Driver tests (hardware ones are off by default):

```sh
cargo test                # unit tests only, the mouse is not touched
cargo test -- --ignored   # plus tests on the real device
```

Battery from the console:

```sh
cargo run -p openshark-driver --example battery
```

Reset the buttons to factory functions from the console:

```sh
cargo run -p openshark-driver --example buttons
```

## Project structure

| Path                          | Purpose                                                     |
| ----------------------------- | ----------------------------------------------------------- |
| `crates/openshark-driver`     | low-level USB driver (libusb), no GUI                       |
| `src-tauri`                   | Tauri backend: tray, commands, battery polling              |
| `src/` + `static/`            | settings window GUI on SvelteKit                            |
| `udev/99-attack-shark-r1.rules` | device permissions for a regular user                     |

## Protocol

Device: VID `0x1d57`, PID `0xfa60` (2.4G) / `0xfa61` (wired), interface 2.

- Battery: interrupt-IN EP `0x83`, byte `4` of the report × 10 = percent
- Configuration (DPI, polling rate, timers): control transfer `SET_REPORT`
  (`0x21/0x09`, wValue `0x304`–`0x306`); the ACK comes back on the same EP `0x83`
  with `buf[2] == 0x50`
- Button table: feature report `0x08`, 59 bytes: `08 3b 01` + 18 slots of
  3 bytes each + checksum (`sum of bytes[2..58] - 1` in the last byte)

The protocol implementation in Odin served as the base:
[xb-bx/attack-shark-r1-driver](https://github.com/xb-bx/attack-shark-r1-driver).
The button table format comes from the Attack Shark protocol family
(report `0x08`).

> **Important:** the mouse must be woken up (shake it), otherwise reports are
> not ACKed and it looks like "the mouse does not respond".

## Notes

- **NVIDIA + Wayland:** WebKitGTK fails with `Error 71 (Protocol error)` when
  the window is shown. `src-tauri/src/main.rs` sets
  `__NV_DISABLE_EXPLICIT_SYNC=1` automatically when an NVIDIA driver is found,
  nothing has to be configured by hand.
- The window is taller than the visible area (about 1100px of content): the
  Apply button and the status bar are pinned to the bottom, the rest scrolls.
- Do not run the hardware tests (`--ignored`) while playing a game:
  `apply_config` rewrites the mouse settings.

## Status

- [x] Battery reading, tray indicator
- [x] Settings window (DPI, polling rate, timers), saved to JSON
- [x] Factory button table restore

**License:** [GPL-3.0](LICENSE).
