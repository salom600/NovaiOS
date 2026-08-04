# NovaiOS Roadmap

This document tracks what's done, what's in flight, and what's planned.
Items marked **done** ship in the latest ISO build.

## v0.1 — Bootable live ISO   *(in progress)*

- [x] Repository structure + workspace
- [x] Kernel config with `CONFIG_RUST=y`
- [x] Rust kernel module skeleton (`novai_drv`)
- [x] `novai-init` — PID 1 with live overlay support
- [x] `novai-services` — TOML unit supervisor
- [x] `novai-shell` — rescue REPL
- [x] `novai-coreutils` — Rust coreutils facade
- [x] `novai-pkg` — pacman wrapper + store front-end
- [x] `novai-comp` — Smithay-based compositor (stub renderer)
- [x] `novai-panel` — iced top bar
- [x] `novai-launcher` — iced launcher + store
- [x] `novai-settings` — iced settings GUI
- [x] `novai-lock` — iced greeter / locker
- [x] GitHub Actions pipeline (build + auto-fix + nightly)
- [x] ISO build script (dracut + xorriso)
- [ ] **First green CI run** — pending first push to `main`
- [ ] Verify boot in QEMU + VirtualBox

## v0.2 — Real compositor

- [ ] Replace stub renderer with Smithay GLES2 + DRM backend
- [ ] Implement xdg-shell (toplevel + popup)
- [ ] Implement wlr-layer-shell (for novai-panel)
- [ ] Implement wlr-output-management (for novai-settings Display tab)
- [ ] Idle inhibit + screen-share via xdg-desktop-portal
- [ ] Multi-monitor with per-output scale
- [ ] Touchpad gestures (3-finger swipe to switch workspace)
- [ ] Hardware cursor + atomic page-flip

## v0.3 — Polish & ecosystem

- [ ] Signed remote package index for the store
- [ ] One-click install with progress UI
- [ ] Update notifications
- [ ] Theme store (wallpapers + accent colours + icon packs)
- [ ] Notification daemon (mako-style, Rust)
- [ ] Screenshot / screen-recorder (Rust, pipewire)
- [ ] Built-in browser start page (NovaiOS-branded Firefox)
- [ ] Bluetooth UI
- [ ] Power statistics

## v0.4 — Tiling + productivity

- [ ] Split-screen tiling mode (Super+arrow)
- [ ] Workspace grid (2x2)
- [ ] Window rules (per-app workspace / floating / fullscreen)
- [ ] Quick Settings dropdown (Wi-Fi / Bluetooth / volume / perf mode)
- [ ] Search everything (files + apps + settings + web)
- [ ] Global shortcuts (configurable)

## v0.5 — Installer

- [ ] GUI installer (`novai-installer`) written in iced
- [ ] Disk partitioning UI
- [ ] Encryption (LUKS) support
- [ ] Dual-boot detection (Windows / other Linux)
- [ ] Time zone / locale / keyboard selection
- [ ] User account creation
- [ ] Post-install driver setup (NVIDIA / Broadcom / Wi-Fi)

## v1.0 — Stable

- [ ] ARM64 (aarch64) image
- [ ] Signed ISOs (GPG)
- [ ] Atomic A/B updates (NovaiOS-specific, not ostree)
- [ ] Stable API for third-party applets
- [ ] LTS kernel branch (6.12) and rolling kernel branch (latest)
- [ ] First public release announcement

## Beyond 1.0

- [ ] NovaiOS Phone (mobile compositor based on the same `novai-comp`)
- [ ] Built-in AI assistant (local, on-device, Rust + candle)
- [ ] Cloud sync for settings + dotfiles (end-to-end encrypted)
- [ ] First-class gaming mode (MangoHud integration, FSR, gamescope)
- [ ] Container-based app sandboxing (no Flatpak dependency)

## How to move an item up

Open a discussion on GitHub. The fastest way to get a feature is to
submit a PR for it — code wins arguments.
