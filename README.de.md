# OpenShark-R1

Treiber und Tray-Utility für die Maus **Attack Shark R1** unter Linux.

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-green)](LICENSE)
![Platform: Linux](https://img.shields.io/badge/platform-Linux-lightgrey)

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh.md) | [Español](README.es.md) | **Deutsch** | [Français](README.fr.md)

Im Tray liegen zwei Icons: das Maus-Icon und der Ladestand (Minute für Minute
aktualisiert). Das Einstellungsfenster öffnet beim Start; Schließen verbirgt
es im Tray, über den Punkt **Einstellungen...** öffnet es erneut.

<details>
<summary><b>Screenshot</b></summary>

![Einstellungsfenster von OpenShark R1](static/screenshot.png)

</details>

## Funktionen

- Ladestand im Tray (2.4G-Empfänger), Farbanzeige nach Akkustand
- Einstellungsfenster: Abtastfrequenz, 6 DPI-Stufen und die aktive Stufe,
  Sleep-Timer, Klickreaktion, ripple control / angle snap
- **Tastenwiederherstellung**: schreibt die Werks-Tabelle zurück
  (behebt "Klicks tun nichts", wenn Tasten auf nichts gemappt sind)
- Einstellungen liegen in `~/.config/openshark-r1/config.json`
- Läuft als normaler Benutzer (über eine udev-Regel)
- Rust-Backend auf libusb (`rusb`), GUI mit Tauri 2 + Svelte

## Installation

Pakete aus dem Quellcode bauen (Abhängigkeiten siehe [Entwicklung](#entwicklung)):

```sh
pnpm install
pnpm tauri build
```

Die Artefakte landen in `target/release/bundle/`: deb, rpm, AppImage.

udev-Regel einmalig von root installieren, sonst ist die Maus nur für root
erreichbar:

```sh
sudo cp udev/99-attack-shark-r1.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --action=change --subsystem-match=usb
```

## Entwicklung

Abhängigkeiten (Arch/CachyOS):

```sh
sudo pacman -S --needed webkit2gtk-4.1 libayatana-appindicator librsvg \
    gtk3 pkgconf base-devel libusb rustup
```

Die udev-Regel wird einmalig installiert, siehe [Installation](#installation).

Start im Entwicklungsmodus:

```sh
pnpm install
pnpm tauri dev
```

Release-Build:

```sh
pnpm tauri build
```

### AppImage (Arch/CachyOS)

Der Cache, den Tauri für AppImage lädt, bricht auf frischen Systemen: das
alte linuxdeploy scheitert mit seinem `strip` an `.relr.dyn`, und das
gtk-Plugin erzwingt `GDK_BACKEND=x11`, wodurch das Fenster auf NVIDIA + Wayland
leer bleibt. Cache einmalig vorbereiten:

```sh
sudo pacman -S --needed patchelf
pnpm tauri build || true   # füllt der Cache; strip kann hier scheitern, das ist ok
bash scripts/appimage-fix.sh
pnpm tauri build
```

Nach dem Leeren von `~/.cache/tauri` die Schritte mit `appimage-fix.sh`
wiederholen.

### Tests

Treiber-Tests (Hardware-Tests sind standardmäßig aus):

```sh
cargo test                # nur Unit-Tests, die Maus bleibt unberührt
cargo test -- --ignored   # + Tests auf dem echten Gerät
```

Akku aus der Konsole prüfen:

```sh
cargo run -p openshark-driver --example battery
```

Tasten auf Werksfunktionen zurücksetzen:

```sh
cargo run -p openshark-driver --example buttons
```

## Projektstruktur

| Pfad                           | Zweck                                                   |
| ------------------------------ | ------------------------------------------------------- |
| `crates/openshark-driver`      | USB-Treiber auf Low-Level (libusb), ohne GUI            |
| `src-tauri`                    | Tauri-Backend: Tray, Kommandos, Akku-Abfrage            |
| `src/` + `static/`             | GUI des Einstellungsfensters mit SvelteKit              |
| `udev/99-attack-shark-r1.rules` | Gerätezugriff für einen normalen Benutzer              |

## Protokoll

Gerät: VID `0x1d57`, PID `0xfa60` (2.4G) / `0xfa61` (kabelgebunden), Interface 2.

- Akku: interrupt-IN EP `0x83`, Byte `4` des Reports × 10 = Prozent
- Konfiguration (DPI, Abtastfrequenz, Timer): control transfer `SET_REPORT`
  (`0x21/0x09`, wValue `0x304`–`0x306`); das ACK kommt auf demselben EP `0x83`
  mit `buf[2] == 0x50`
- Tastentabelle: Feature-Report `0x08`, 59 Bytes: `08 3b 01` + 18 Slots zu
  3 Bytes + Prüfsumme (`Summe von bytes[2..58] - 1` im letzten Byte)

Die Protokollimplementierung in Odin diente als Grundlage:
[xb-bx/attack-shark-r1-driver](https://github.com/xb-bx/attack-shark-r1-driver).
Das Format der Tastentabelle stammt aus der Attack-Shark-Protokollfamilie
(Report `0x08`).

> **Wichtig:** Die Maus muss aufgeweckt werden (schütteln), sonst werden die
> Reports nicht ACKed und es sieht aus wie "die Maus antwortet nicht".

## Hinweise

- **NVIDIA + Wayland:** WebKitGTK fällt mit `Error 71 (Protocol error)` auf,
  wenn das Fenster angezeigt wird. `src-tauri/src/main.rs` setzt
  `__NV_DISABLE_EXPLICIT_SYNC=1` automatisch, wenn ein NVIDIA-Treiber gefunden
  wird, manuell ist nichts zu konfigurieren.
- Das Fenster ist höher als der sichtbare Bereich (etwa 1100px Inhalt): die
  Schaltfläche "Anwenden" und die Statusleiste sind unten verankert, der Rest
  scrollt.
- Hardware-Tests (`--ignored`) nicht während eines Spiels ausführen:
  `apply_config` überschreibt die Mauseinstellungen.

## Status

- [x] Akku-Auslesen, Anzeige im Tray
- [x] Einstellungsfenster (DPI, Abtastfrequenz, Timer), Speicherung als JSON
- [x] Wiederherstellung der Werks-Tastentabelle

**Lizenz:** [GPL-3.0](LICENSE).
