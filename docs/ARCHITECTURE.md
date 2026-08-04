# NovaiOS Architecture

This document explains the design of every layer in NovaiOS, from the boot
sector up to the desktop shell. It is the canonical reference for anyone
contributing to the project.

## 1. Boot flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│  UEFI firmware                                                          │
│    ↓  loads                                                             │
│  /EFI/BOOT/BOOTX64.EFI   (UKI: kernel + initramfs + cmdline, signed)    │
│    ↓  or                                                                 │
│  isolinux.bin (BIOS)   →  vmlinuz-novai + initramfs-novai.img            │
└─────────────────────────────────────────────────────────────────────────┘
                                  ↓
┌─────────────────────────────────────────────────────────────────────────┐
│  initramfs (dracut)                                                     │
│    1. 95novai module mounts the ISO by CDLABEL=NOVAI_ISO                │
│    2.  mounts filesystem.squashfs at /run/novai/sqfs                    │
│    3.  exec /init  →  novai-init                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                  ↓
┌─────────────────────────────────────────────────────────────────────────┐
│  novai-init (PID 1, Rust)                                               │
│    1. mount /proc /sys /dev /run /tmp                                   │
│    2. parse /proc/cmdline: novai.live=1, novai.root=..., novai.init=... │
│    3. mount overlay over squashfs (lowerdir=sqfs, upperdir=tmpfs)       │
│    4. pivot_root into the new root                                      │
│    5. execve(/sbin/init)                                                │
└─────────────────────────────────────────────────────────────────────────┘
                                  ↓
┌─────────────────────────────────────────────────────────────────────────┐
│  /sbin/init  →  either systemd (default) or novai-services              │
│    - brings up NetworkManager, seatd, dbus, pipewire                    │
│    - launches novai-comp (Wayland compositor) as user `novai`           │
│    - novai-comp autostarts novai-panel, novai-lock (daemon)             │
└─────────────────────────────────────────────────────────────────────────┘
                                  ↓
┌─────────────────────────────────────────────────────────────────────────┐
│  Desktop (Wayland)                                                      │
│    - novai-panel   — layer-surface top bar                              │
│    - novai-launcher — full-screen overlay (Super key) + Store tab       │
│    - novai-settings — system config GUI                                 │
│    - novai-lock    — greeter / idle lock                                │
└─────────────────────────────────────────────────────────────────────────┘
```

## 2. Kernel layer

NovaiOS uses the upstream Linux kernel (currently 6.12 LTS) with the
following non-default choices:

| Config                   | Why                                                |
|--------------------------|----------------------------------------------------|
| `CONFIG_RUST=y`          | enable Rust-for-Linux modules                      |
| `CONFIG_RUST_MISCDEV=m`  | build the upstream sample so our module links      |
| `CONFIG_PREEMPT=y`       | low-latency desktop responsiveness                 |
| `CONFIG_HZ_1000=y`       | 1000 Hz tick for smooth UI                         |
| `CONFIG_CPU_FREQ_DEFAULT_GOV_SCHEDUTIL=y` | EPP-style governor               |
| `CONFIG_DRM_AMDGPU=y`, `DRM_I915=y`, `DRM_NOUVEAU=y` | all 3 GPU vendors |
| `CONFIG_OVERLAY_FS=y`, `SQUASHFS=y`       | live ISO rootfs                     |
| `CONFIG_ZSWAP=y`, `ZSWAP_COMPRESSOR_DEFAULT_ZSTD=y`    | compressed swap       |
| `CONFIG_KVM=y`, `VIRTIO=*`, `HYPERV=*`, `VMWARE_*`     | full guest support   |

The full configuration lives in `kernel/config-novai-x86_64`.

### 2.1 novai_drv Rust module

`kernel/rust-modules/src/lib.rs` is a real Rust-for-Linux module. It:

- Registers `/dev/novai` as a misc char device.
- Exposes a small JSON-ish telemetry line on read.
- Will grow sysfs hooks (`/sys/kernel/novai/perf_mode`) once the
  in-kernel `sysfs` Rust API stabilises upstream.

## 3. Userland layer

Each binary lives in its own crate under `userland/` or `desktop/`. They
share the workspace `Cargo.toml` for dependency pinning.

### 3.1 novai-init

- Reads `/proc/cmdline` (or `NOVAI_CMDLINE` env for testing).
- Mounts the API filesystems (`/proc`, `/sys`, `/dev`, etc.) via the
  `nix` crate's `mount(2)` wrapper.
- In live mode: discovers the squashfs on the ISO (by CDLABEL, by path,
  or by kernel cmdline hint), mounts it read-only, overlays a tmpfs on
  top, then `pivot_root(2)`s into it.
- In installed mode: mounts the root block device specified by
  `novai.root=<dev>:<fstype>:<opts>`.
- Execs `/sbin/init` (or whatever `novai.init=` specifies).

### 3.2 novai-services

A small supervised process manager reading TOML unit files from
`/etc/novai/services/*.toml`. Each unit declares `exec`, `restart`
policy, `user`, and `env`. The supervisor restarts units on policy
and shuts down cleanly on SIGTERM.

The unit format is intentionally NOT a clone of systemd's — it is small
and TOML-native so it is easy to author from a Rust GUI.

### 3.3 novai-shell

A minimal REPL for the rescue console. Built-ins: `cd`, `pwd`, `exit`,
`export`, `echo`, `set`, `alias`, `history`, `source`, `which`, `help`.
External commands resolved via `which`. The user's default `$SHELL` in
the live ISO is Nushell (`/usr/bin/nu`).

### 3.4 novai-coreutils

A small facade. When `uutils-coreutils` is installed (it is, by default,
in the live ISO) most commands proxy to it; for the boot shell we
implement `cat`, `ls`, `rm`, `mkdir`, `pwd`, `tail`, `head`, `cp`, `mv`,
`uptime`, `free` in pure Rust so the system is usable even if the
`uutils` package is removed.

### 3.5 novai-pkg

- Backend: `pacman` (proven, signed, fast).
- Frontend: friendly Rust CLI + a programmatic mode used by the desktop
  store (`--no-confirm --from-store <slug>`).
- The desktop store is a curated catalog (see `novai-launcher/src/main.rs`)
  that will eventually fetch a signed remote index.

## 4. Desktop layer

### 4.1 novai-comp

Built on [Smithay](https://github.com/Smithay/smithay). In the first
release it boots as a stub renderer that clears the framebuffer; in
production it will use Smithay's `backend_drm` + `renderer_glow` for a
real OpenGL ES compositor with `xdg-shell` + `wlr-layer-shell` + 
`wlr-output-management` protocols.

Configuration: `/etc/novai/comp.toml` or `$XDG_CONFIG_HOME/novai/comp.toml`.

### 4.2 novai-panel

Top-of-screen `iced` window, 36 px tall, with:

- Workspace switcher (1..N, click to switch).
- Active window title (read from Wayland via a future IPC).
- System tray placeholders.
- Clock + date (`chrono`).
- Battery / network indicators (read from `/sys/class/power_supply/BAT*`
  and `nmcli`).

### 4.3 novai-launcher

Full-screen iced window with two modes:

- **Launcher** (`novai-launcher`): fuzzy search installed apps, Enter to
  launch.
- **Store** (`novai-launcher --store`): curated catalog with one-click
  `[Install]` buttons that spawn `novai-pkg install --no-confirm <slug>`.

### 4.4 novai-settings

Tabbed settings GUI: Appearance / Display / Sound / Network / Power /
Users / About. The Power tab sets `perf_mode` (balanced / performance /
powersave) via `/sys/kernel/novai/perf_mode` when the kernel module
exposes it, otherwise via `cpupower frequency-set -g`.

### 4.5 novai-lock

Greeter + screen locker. Lists human users from `/etc/passwd`, accepts a
password, and spawns `/bin/login -f <user>` on success. A `--daemon` mode
listens on `/run/novai/lock.sock` so the compositor can request lock on
idle.

## 5. ISO layer

The build pipeline is three stages, each in its own script:

| Stage          | Script                  | Output                                |
|----------------|-------------------------|---------------------------------------|
| 0 — Kernel     | `scripts/build-kernel.sh`  | `build/out-kernel/vmlinuz-novai`   |
| 1 — Userland   | `scripts/build-userland.sh`| `build/userland/bin/*`            |
| 2 — ISO        | `scripts/build-iso.sh`     | `build/out/novaios-*.iso`         |

Stage 2 runs inside an `archlinux:latest` container so it has `pacstrap`,
`dracut`, `xorriso`, and the `arch-install-scripts` available.

The final ISO layout:

```
/
├── boot/
│   ├── vmlinuz-novai
│   ├── initramfs-novai.img
│   └── loader/entries/novai.conf
├── EFI/BOOT/BOOTX64.EFI      (UKI for UEFI boot)
├── isolinux/isolinux.cfg     (BIOS boot)
├── loader/loader.conf
└── novai/live/
    └── filesystem.squashfs   (zstd-compressed rootfs)
```

## 6. CI/CD layer

See [`.github/workflows/`](../.github/workflows/) and the README's
"Self-healing CI" section.
