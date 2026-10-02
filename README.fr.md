# OpenShark-R1

Pilote et utilitaire de tray pour la souris **Attack Shark R1** sous Linux.

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-green)](LICENSE)
![Platform: Linux](https://img.shields.io/badge/platform-Linux-lightgrey)

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh.md) | [Español](README.es.md) | [Deutsch](README.de.md) | **Français**

Dans le tray, deux icônes : l'icône de la souris et le niveau de batterie
(actualisé une fois par minute). La fenêtre de réglages s'ouvre au démarrage ;
la fermer la cache dans le tray, et l'élément **Réglages...** la rouvre depuis
celui-ci.

<details>
<summary><b>Capture d'écran</b></summary>

![Fenêtre de réglages d'OpenShark R1](static/screenshot.png)

</details>

## Fonctionnalités

- Niveau de batterie dans le tray (récepteur 2.4G), indication par couleur
- Fenêtre de réglages : fréquence de sonde, 6 niveaux de DPI et le niveau
  actif, minuteurs de veille, réponse du clic, ripple control / angle snap
- **Restauration des boutons** : réécrit la table d'affectation d'usine
  (répare "les clics ne font rien" quand les boutons sont mappés vers rien)
- Les réglages sont stockés dans `~/.config/openshark-r1/config.json`
- Fonctionne en tant qu'utilisateur normal (via une règle udev)
- Backend Rust sur libusb (`rusb`), GUI sur Tauri 2 + Svelte

## Installation

Compiler les paquets depuis les sources (dépendances dans
[Développement](#développement)) :

```sh
pnpm install
pnpm tauri build
```

Les artefacts se trouvent dans `target/release/bundle/` : deb, rpm, AppImage.

Installer la règle udev une fois depuis root, sinon la souris n'est
accessible qu'à root :

```sh
sudo cp udev/99-attack-shark-r1.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --action=change --subsystem-match=usb
```

## Développement

Dépendances (Arch/CachyOS) :

```sh
sudo pacman -S --needed webkit2gtk-4.1 libayatana-appindicator librsvg \
    gtk3 pkgconf base-devel libusb rustup
```

La règle udev s'installe une fois, voir [Installation](#installation).

Lancer en mode développement :

```sh
pnpm install
pnpm tauri dev
```

Build de release :

```sh
pnpm tauri build
```

### AppImage (Arch/CachyOS)

Le cache que Tauri télécharge pour l'AppImage casse sur les systèmes récents :
l'ancien linuxdeploy échoue avec son `strip` sur `.relr.dyn`, et le plugin gtk
force `GDK_BACKEND=x11`, ce qui laisse la fenêtre vide sur NVIDIA + Wayland.
Préparer le cache une fois :

```sh
sudo pacman -S --needed patchelf
pnpm tauri build || true   # remplit le cache ; strip peut échouer, c'est normal
bash scripts/appimage-fix.sh
pnpm tauri build
```

Après avoir vidé `~/.cache/tauri`, refaire les étapes avec `appimage-fix.sh`.

### Tests

Tests du pilote (les tests matériel sont désactivés par défaut) :

```sh
cargo test                # tests unitaires seulement, la souris n'est pas touchée
cargo test -- --ignored   # + tests sur le périphérique réel
```

Batterie depuis la console :

```sh
cargo run -p openshark-driver --example battery
```

Remettre les boutons sur les fonctions d'usine depuis la console :

```sh
cargo run -p openshark-driver --example buttons
```

## Structure du projet

| Chemin                          | Rôle                                                    |
| ------------------------------- | ------------------------------------------------------- |
| `crates/openshark-driver`       | pilote USB bas niveau (libusb), sans GUI                |
| `src-tauri`                     | backend Tauri : tray, commandes, sonde de batterie      |
| `src/` + `static/`              | GUI de la fenêtre de réglages sur SvelteKit             |
| `udev/99-attack-shark-r1.rules` | droits d'accès à l'appareil pour un utilisateur normal  |

## Protocole

Appareil : VID `0x1d57`, PID `0xfa60` (2.4G) / `0xfa61` (filé), interface 2.

- Batterie : interrupt-IN EP `0x83`, octet `4` du rapport × 10 = pourcentage
- Configuration (DPI, fréquence de sonde, minuteurs) : control transfer
  `SET_REPORT` (`0x21/0x09`, wValue `0x304`–`0x306`) ; l'ACK revient sur le
  même EP `0x83` avec `buf[2] == 0x50`
- Table des boutons : rapport de caractéristiques `0x08`, 59 octets :
  `08 3b 01` + 18 emplacements de 3 octets + somme de contrôle
  (`somme de bytes[2..58] - 1` dans le dernier octet)

L'implémentation du protocole en Odin a servi de base :
[xb-bx/attack-shark-r1-driver](https://github.com/xb-bx/attack-shark-r1-driver).
Le format de la table des boutons vient de la famille de protocoles
Attack Shark (rapport `0x08`).

> **Important :** la souris doit être réveillée (secouez-la), sinon les rapports
> ne sont pas ACKés et cela ressemble à "la souris ne répond pas".

## Remarques

- **NVIDIA + Wayland :** WebKitGTK échoue avec `Error 71 (Protocol error)` à
  l'affichage de la fenêtre. `src-tauri/src/main.rs` pose
  `__NV_DISABLE_EXPLICIT_SYNC=1` automatiquement si un pilote NVIDIA est
  trouvé, rien à configurer à la main.
- La fenêtre dépasse la zone visible (environ 1100px de contenu) : le bouton
  Appliquer et la barre d'état sont fixés en bas, le reste défile.
- Ne lancez pas les tests matériels (`--ignored`) pendant une partie :
  `apply_config` réécrit les réglages de la souris.

## État

- [x] Lecture de la batterie, indicateur dans le tray
- [x] Fenêtre de réglages (DPI, fréquence de sonde, minuteurs), sauvegarde en JSON
- [x] Restauration de la table des boutons d'usine

**Licence :** [GPL-3.0](LICENSE).
