# NovaiOS

**A modern, lightweight, Rust-native Linux distribution.**

NovaiOS takes the upstream Linux kernel — with Rust-for-Linux enabled — and
wraps it in a fully Rust userland: an init system, shell, coreutils, package
manager, Wayland compositor, and desktop shell. Every component beyond the
kernel's C core is written in Rust, compiled in CI, and delivered as a
bootable ISO.

> **Status:** early preview. ISOs are built automatically by GitHub Actions
> on every push to `main` and on a weekly schedule. See the
> [Actions tab](https://github.com/salom600/NovaiOS/actions) for live builds.

---

## Why Rust?

Rust is the 2026 gold standard for systems programming:

- **Memory safety** without a garbage collector — eliminates entire
  classes of CVEs (use-after-free, buffer overflow, data races).
- **Zero-cost abstractions** let us write high-level desktop code that
  compiles down to machine code as tight as hand-written C.
- **Fearless concurrency** makes the service manager and compositor
  thread-safe by construction.
- **Strong type system** turns runtime crashes into compile errors.

The Linux kernel itself has accepted Rust modules since 6.1 (October 2022),
and NovaiOS ships a custom kernel module written in Rust that talks to our
userland service manager.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                      NovaiOS ISO                              │
├──────────────────────────────────────────────────────────────┤
│ Boot      systemd-boot + dracut initramfs                     │
├──────────────────────────────────────────────────────────────┤
│ Kernel    Linux 6.12 LTS  (CONFIG_RUST=y)                     │
│           + novai_drv.ko  (Rust, in-tree-style module)        │
├──────────────────────────────────────────────────────────────┤
│ Init      novai-init (PID 1, Rust)                            │
│           mounts /proc /sys /dev, finds squashfs,             │
│           overlayfs live root, pivot_root, exec /sbin/init    │
├──────────────────────────────────────────────────────────────┤
│ Services  novai-services  (Rust supervisor)                   │
│           OR systemd (fallback)                               │
├──────────────────────────────────────────────────────────────┤
│ Userland  uutils-coreutils  · Nushell · bat · fd · rg · eza   │
│           novai-shell (rescue shell)                          │
│           novai-pkg  (pacman backend + store front-end)       │
├──────────────────────────────────────────────────────────────┤
│ Display   novai-comp  (Wayland compositor on Smithay)         │
│ Desktop   novai-panel · novai-launcher · novai-settings       │
│           · novai-lock  (greeter / locker)                    │
├──────────────────────────────────────────────────────────────┤
│ Drivers   Full Linux driver tree (AMD, NVIDIA, Intel,         │
│           nouveau, virtio, Hyper-V, VMware, etc.)             │
│ Firmware  linux-firmware (for amdgpu/i915/nouveau)            │
└──────────────────────────────────────────────────────────────┘
```

For more detail see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## What's in this repo?

```
novaios/
├── .github/workflows/        # CI/CD pipelines (build ISO, auto-fix, nightly)
│   ├── build-iso.yml         # Main pipeline: kernel + userland + ISO
│   ├── auto-fix.yml          # Self-healing: parse logs, patch, retry
│   ├── nightly.yml           # Auto-tag a nightly release
│   └── ci.yml                # Fast PR gate (fmt, clippy, test)
├── kernel/
│   ├── config-novai-x86_64   # Full kernel config (Rust + drivers)
│   └── rust-modules/         # novai_drv — Rust kernel module
├── userland/
│   ├── novai-init/           # PID 1, mounts + pivot_root
│   ├── novai-services/       # Service supervisor
│   ├── novai-shell/          # Boot / rescue shell
│   ├── novai-coreutils/      # cat/ls/rm/... in Rust
│   └── novai-pkg/            # Package manager (pacman + store)
├── desktop/
│   ├── novai-comp/           # Wayland compositor (Smithay)
│   ├── novai-panel/          # Top bar (iced)
│   ├── novai-launcher/       # App launcher + store (iced)
│   ├── novai-settings/       # System settings (iced)
│   ├── novai-lock/           # Greeter / locker (iced)
│   └── novai-theme/theme.css # Shared palette
├── iso/profile/              # Rootfs overlay (services, entries)
├── scripts/
│   ├── build-kernel.sh       # Stage 0
│   ├── build-userland.sh     # Stage 1
│   ├── build-iso.sh          # Stage 2
│   └── auto-fix.py           # CI self-healing
├── docs/
│   ├── ARCHITECTURE.md
│   ├── BUILD.md
│   └── ROADMAP.md
└── Cargo.toml                # Workspace root
```

---

## Building locally

The whole thing builds inside GitHub Actions, but you can reproduce it
locally on any recent Ubuntu / Arch machine:

```bash
# 1. Build all Rust userland binaries (needs: cargo, libudev, libseat, libegl)
./scripts/build-userland.sh

# 2. Build the kernel with Rust enabled (needs: clang, llvm, lld, bc, flex, bison)
./scripts/build-kernel.sh

# 3. Assemble the bootable ISO (needs: archlinux + pacstrap + xorriso)
#    Easiest in a container:
docker run --rm --privileged -v "$PWD:/work" -w /work archlinux:latest \
    bash -c 'pacman -Syu --noconfirm base base-devel arch-install-scripts \
        dracut squashfs-tools xorriso syslinux linux-firmware rustup git wget &&
     rustup default stable && ./scripts/build-iso.sh'
```

See [docs/BUILD.md](docs/BUILD.md) for the full deep-dive.

---

## Downloading a pre-built ISO

Every push to `main` and every Sunday at 03:00 UTC produces a fresh ISO:

1. Open the [Actions tab](https://github.com/salom600/NovaiOS/actions).
2. Click the most recent **build-iso** run.
3. Scroll to **Artifacts** and download `novai-iso`.

Tagged releases (e.g. `v0.1.0`) appear on the
[Releases page](https://github.com/salom600/NovaiOS/releases).

Burn it to USB with:

```bash
sudo dd if=novaios-*.iso of=/dev/sdX bs=4M conv=fsync status=progress
```

Boot it in **Live mode** (default) or **Installation mode** from the
boot menu.

---

## Running in a VM

The ISO is tested to boot in:

- **QEMU/KVM**:  `qemu-system-x86_64 -enable-kvm -m 4G -cdrom novaios.iso`
- **VirtualBox**: 4 GB RAM, 32 GB disk, EFI enabled
- **VMware Workstation**: same as VirtualBox

For VirtualBox, the `vboxguest` modules are loaded automatically.

---

## Self-healing CI

When the build pipeline fails, an `auto-fix` workflow runs and applies
rules from `scripts/auto-fix.py`:

| Rule                       | Trigger                                    | Fix                                       |
|----------------------------|--------------------------------------------|-------------------------------------------|
| `missing-system-dep`       | `command not found: <pkg>`                 | add `<pkg>` to `pacstrap` call            |
| `rust-missing-use`         | `error[E0433]: unresolved module <crate>`  | prepend `use <crate>;`                    |
| `rust-feature-missing`     | `error: cargo feature <f> not enabled`     | enable `<f>` in `Cargo.toml`              |
| `kernel-config-missing`    | `warning: CONFIG_<SYM> not set`            | add `--enable CONFIG_<SYM>` to build-kernel.sh |
| `wget-404`                 | kernel download 404                        | bump `KVER` to a newer LTS                |
| `cargo-lock-drift`         | `lock file needs to be updated`            | `cargo update`                            |

If a rule fires, `novai-bot` commits & pushes the patch, which re-triggers
`build-iso`. A loop guard prevents the bot from re-triggering itself when
the last commit was already authored by `novai-bot` — at that point the bot
opens a GitHub issue tagged `auto-fix-skipped` for human triage.

---

## Roadmap

See [docs/ROADMAP.md](docs/ROADMAP.md). Highlights:

- **v0.1** (this release) — bootable ISO, live session, working installer skeleton.
- **v0.2** — Wayland compositor with real KMS/DRM rendering, panel, launcher.
- **v0.3** — novai-pkg store with signed remote index + one-click install.
- **v0.4** — Split-screen tiling, gesture support, theme store.
- **v1.0** — Stable API, ARM64 image, signed ISOs, optional in-place upgrades.

---

## Contributing

Pull requests are welcome. Please:

1. Run `cargo fmt --all` and `cargo clippy --workspace` before pushing.
2. Keep commits atomic — the auto-fix bot relies on clean diffs.
3. Add a unit test for any non-trivial Rust code.

For bigger design discussions, open a GitHub Discussion first.

---

## License

Triple-licensed for maximum compatibility:

- **MIT**        ([LICENSE-MIT](LICENSE-MIT))
- **Apache-2.0** ([LICENSE-APACHE](LICENSE-APACHE))
- **GPL-3.0+**   ([LICENSE-GPL](LICENSE-GPL))

The Linux kernel module in `kernel/rust-modules/` is GPL-2.0-only to match
the kernel's linking requirements.

---

## Acknowledgements

NovaiOS stands on the shoulders of giants:

- [Linux kernel](https://kernel.org) + [Rust for Linux](https://rust-for-linux.com)
- [Smithay](https://github.com/Smithay/smithay) — Rust Wayland library
- [Iced](https://github.com/iced-rs/iced) — Rust GUI toolkit
- [uutils-coreutils](https://github.com/uutils/coreutils) — Rust coreutils
- [Nushell](https://www.nushell.sh) — Rust shell
- [Arch Linux](https://archlinux.org) — base distribution and pacman
- [dracut](https://github.com/dracutdevs/dracut) — initramfs generator

NovaiOS is an independent project and is not affiliated with any of the above.
