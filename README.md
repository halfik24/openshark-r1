# OpenShark-R1

Драйвер и трей-утилита для мыши **Attack Shark R1** для Linux.

В трее две иконки: значок мыши и уровень заряда (обновляется раз в минуту).
Окно настроек открывается при старте; закрытие прячет его в трей, оттуда же
пункт **«Настройки…»** открывает заново.

<details>
<summary><b>Скриншот</b></summary>

![Окно настроек OpenShark R1](static/screenshot.png)

</details>

## Возможности

- Уровень заряда батареи в трее (2.4G-приёмник), цветовая индикация
- Окно настроек: частота опроса, 6 ступеней DPI и активная ступень,
  таймеры сна, отклик кнопки, ripple control / angle snap
- **Восстановление кнопок** — возвращает заводскую таблицу переназначения
  (чинит «клики не работают», когда кнопки замаплены в пустоту)
- Настройки хранятся в `~/.config/openshark-r1/config.json`
- Работает от обычного пользователя (через udev-правило)
- Rust-бэкенд поверх libusb (`rusb`), GUI — Tauri 2 + Svelte

<details>
<summary><b>Установка</b></summary>

Сборка пакетов из исходников (зависимости — в разделе «Разработка»):

```sh
pnpm install
pnpm tauri build
```

Готовые артефакты окажутся в `target/release/bundle/` — deb, rpm, AppImage.

udev-правило, один раз от root (иначе доступ к мыши только у root'а):

```sh
sudo cp udev/99-attack-shark-r1.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --action=change --subsystem-match=usb
```

</details>

<details>
<summary><b>Разработка и сборка</b></summary>

Зависимости (Arch/CachyOS):

```sh
sudo pacman -S --needed webkit2gtk-4.1 libayatana-appindicator librsvg \
    gtk3 pkgconf base-devel libusb rustup
```

udev-правило — один раз, см. раздел «Установка».

Запуск в разработке:

```sh
pnpm install
pnpm tauri dev
```

Сборка релиза:

```sh
pnpm tauri build
```

### AppImage (Arch/CachyOS)

Кеш, который Tauri скачивает для AppImage, на свежих системах ломается:
linuxdeploy старой сборки падает своим `strip` на `.relr.dyn`, а gtk-плагин
прописывает форс `GDK_BACKEND=x11` — на NVIDIA+Wayland окно остаётся пустым.
Один раз подготовить кеш:

```sh
sudo pacman -S --needed patchelf
pnpm tauri build || true   # пополняет кеш; на strip может упасть — это нормально
bash scripts/appimage-fix.sh
pnpm tauri build
```

После очистки `~/.cache/tauri` шаги с `appimage-fix.sh` повторить.

Тесты драйвера (хардварные по умолчанию выключены):

```sh
cargo test                # только юнит-тесты, мышь не трогает
cargo test -- --ignored   # + тесты на реальном устройстве
```

Проверка заряда из консоли:

```sh
cargo run -p openshark-driver --example battery
```

Сброс кнопок в заводские функции из консоли:

```sh
cargo run -p openshark-driver --example buttons
```

</details>

<details>
<summary><b>Структура проекта</b></summary>

| Путь                       | Назначение                                    |
| -------------------------- | --------------------------------------------- |
| `crates/openshark-driver`  | низкоуровневый USB-драйвер (libusb), без GUI   |
| `src-tauri`                | Tauri-бэкенд: трей, команды, опрос батареи     |
| `src/` + `static/`          | GUI на SvelteKit — окно настроек               |
| `udev/99-attack-shark-r1.rules` | права доступа к устройству для обычного пользователя |

</details>

<details>
<summary><b>Протокол</b></summary>

Устройство: VID `0x1d57`, PID `0xfa60` (2.4G) / `0xfa61` (провод), interface 2.

- Батарея — interrupt-IN EP `0x83`, байт `4` отчёта × 10 = проценты
- Конфигурация (DPI, polling rate, таймеры) — control transfer `SET_REPORT`
  (`0x21/0x09`, wValue `0x304`–`0x306`); ACK — тот же EP `0x83` с `buf[2] == 0x50`
- Таблица кнопок — feature-отчёт `0x08`, 59 байт: `08 3b 01` + 18 слотов по
  3 байта + контрольная сумма (`сумма bytes[2..58] - 1` в последнем байте)

Реализация протокола на Odin взята за основу:
[xb-bx/attack-shark-r1-driver](https://github.com/xb-bx/attack-shark-r1-driver).
Формат таблицы кнопок — из семейства протоколов Attack Shark (отчёт `0x08`).

> **Важно:** мышь должна быть разбужена (подёргать её), иначе отчёты не
> ACK-ятся — это выглядит как «мышь не отвечает».

</details>

<details>
<summary><b>Замечания</b></summary>

- **NVIDIA + Wayland:** WebKitGTK падает с `Error 71 (Protocol error)` при
  показе окна. `src-tauri/src/main.rs` автоматически выставляет
  `__NV_DISABLE_EXPLICIT_SYNC=1`, если найден драйвер NVIDIA — руками ничего
  настраивать не нужно.
- Окно выше видимой области (контент ~1100px): кнопки «Применить» и статус
  закреплены внизу, основная часть прокручивается.
- Тесты с реальным железом (`--ignored`) не запускайте во время игры:
  `apply_config` переписывает настройки мыши.

</details>

<details>
<summary><b>Статус</b></summary>

- [x] Чтение заряда, индикация в трее
- [x] Окно настроек (DPI, polling rate, таймеры), сохранение в JSON
- [x] Восстановление заводской таблицы кнопок

</details>

**Лицензия:** [GPL-3.0](LICENSE).
