#!/usr/bin/env bash
# Готовит кеш Tauri (~/.cache/tauri) к сборке AppImage на Arch/CachyOS.
# Идемпотентно: повторять после каждой чистки кеша.
#
# Что чинит:
#  1. linuxdeploy из кеша собран старым binutils — его strip падает
#     на SHT_RELR (.relr.dyn) новых системных библиотек. Меняем на continuous.
#  2. gtk-плагин пишет в apprun-hook `export GDK_BACKEND=x11` — на
#     NVIDIA+Wayland рантайм падает на GBM, окно остаётся пустым.
#  3. gtk-плагин find'ом подхватывает неполные vendor-копии gtk-библиотек
#     из /usr/lib/vmware — пропускаем этот каталог.
set -euo pipefail

CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/tauri"
GTK="$CACHE/linuxdeploy-plugin-gtk.sh"
LINUXDEPLOY="$CACHE/linuxdeploy-x86_64.AppImage"
# Старее этой сборки linuxdeploy собран без поддержки SHT_RELR (binutils < 2.39).
MIN_BUILD=300

die() { echo "✗ $*" >&2; exit 1; }

command -v patchelf > /dev/null ||
  die "patchelf не найден: sudo pacman -S patchelf"

[ -f "$GTK" ] && [ -f "$LINUXDEPLOY" ] ||
  die "кеш $CACHE пуст — соберите AppImage один раз (файлы скачаются, даже если сборка упадёт), затем повторите скрипт"

# 1. linuxdeploy → свежая continuous-сборка (--version печатает в stderr)
build=$(grep -oE 'build [0-9]+' <<<"$("$LINUXDEPLOY" --version 2>&1)" |
  grep -oE '[0-9]+' || true)
if [ -z "$build" ] || [ "$build" -lt "$MIN_BUILD" ]; then
  echo "→ linuxdeploy: сборка ${build:-?} → continuous"
  [ -f "$LINUXDEPLOY.bak" ] || mv "$LINUXDEPLOY" "$LINUXDEPLOY.bak"
  URL=https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
  if command -v curl > /dev/null; then
    curl -fL -o "$LINUXDEPLOY" "$URL"
  else
    wget -O "$LINUXDEPLOY" "$URL"
  fi
  chmod +x "$LINUXDEPLOY"
fi

# 2. gtk-плагин: форс GDK_BACKEND=x11 в apprun-hook
if grep -q '^export GDK_BACKEND=x11' "$GTK"; then
  echo "→ gtk-плагин: убран форс GDK_BACKEND=x11"
  sed -i 's|^export GDK_BACKEND=x11|# OpenShark: форс x11 даёт пустое окно на NVIDIA+Wayland\n# &|' "$GTK"
fi

# 3. gtk-плагин: vendor-копии gtk в /usr/lib/vmware
if ! grep -q 'vmware' "$GTK"; then
  echo "→ gtk-плагин: каталог vmware пропускается при поиске библиотек"
  sed -i "s|find \"\$directory\" \\\\(|find \"\$directory\" -path '*/vmware/*' -prune -o \\\\(|" "$GTK"
fi

bash -n "$GTK" || die "gtk-плагин после патча не парсится"
echo "✓ кеш tauri готов: pnpm tauri build"
