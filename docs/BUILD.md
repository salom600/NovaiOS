# Building NovaiOS

There are three ways to build NovaiOS:

1. **Let CI build it** — push to `main` and pick up the ISO from the
   Actions tab. Zero local tooling needed.
2. **Build locally** — useful for development. Needs Ubuntu 22.04+ or
   Arch, ~20 GB disk, ~8 GB RAM.
3. **Build in Docker** — reproduces the CI environment on any Linux host
   with Docker installed.

## Option 1 — CI build (zero setup)

```bash
git clone https://github.com/salom600/NovaiOS.git
cd NovaiOS
git commit --allow-empty -m "trigger build" && git push
# Wait ~25 minutes, then download from:
#   https://github.com/salom600/NovaiOS/actions
```

## Option 2 — Local build

### Prerequisites

```bash
# Ubuntu / Debian
sudo apt install -y build-essential clang llvm lld bc bison flex \
    libncurses-dev libelf-dev libssl-dev cpio kmod wget xz-utils \
    pkg-config libudev-dev libdbus-1-dev libxkbcommon-dev libgbm-dev \
    libinput-dev libseat-dev libegl-dev libgl-dev libwayland-dev \
    gcc-multilib

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
cargo install --locked --version 0.69.5 bindgen-cli

# Arch-specific (for ISO stage)
# Easiest: use a Docker container with archlinux:latest — see option 3.
```

### Build steps

```bash
# 0. Rust userland
./scripts/build-userland.sh

# 1. Kernel
./scripts/build-kernel.sh        # ~25 minutes, cached after first run

# 2. ISO  (run in archlinux container)
docker run --rm --privileged -v "$PWD:/work" -w /work archlinux:latest \
    bash -c 'pacman -Syu --noconfirm base base-devel arch-install-scripts \
        dracut squashfs-tools xorriso syslinux linux-firmware rustup git wget curl which && \
     rustup default stable && \
     ./scripts/build-iso.sh'
```

Output: `build/out/novaios-<date>-x86_64.iso`.

## Option 3 — All-in-Docker

```bash
docker run --rm --privileged -v "$PWD:/work" -w /work archlinux:latest bash <<'EOF'
set -e
pacman -Syu --noconfirm base base-devel arch-install-scripts \
    dracut squashfs-tools xorriso syslinux linux-firmware \
    rustup git wget curl which clang llvm lld pkgconf \
    libxcb libxkbcommon libinput libseat dbus pipewire

rustup default stable
rustup target add x86_64-unknown-linux-gnu

./scripts/build-userland.sh
./scripts/build-kernel.sh
./scripts/build-iso.sh
EOF
```

## Inspecting an existing ISO

```bash
# Loop-mount and inspect
mkdir -p /mnt/iso
sudo mount -o loop build/out/novaios-*.iso /mnt/iso
ls /mnt/iso

# Mount the squashfs
mkdir -p /mnt/rootfs
sudo mount -t squashfs -o ro /mnt/iso/novai/live/filesystem.squashfs /mnt/rootfs
ls /mnt/rootfs
```

## Testing in QEMU

```bash
qemu-system-x86_64 \
    -enable-kvm \
    -m 4G \
    -smp 4 \
    -cdrom build/out/novaios-*.iso \
    -boot d \
    -vga virtio \
    -display gtk
```

For UEFI testing:

```bash
qemu-system-x86_64 \
    -enable-kvm -m 4G -smp 4 \
    -cdrom build/out/novaios-*.iso \
    -bios /usr/share/OVMF/OVMF_CODE.fd \
    -vga virtio -display gtk
```

## Testing in VirtualBox / VMware

1. Create a new VM: Linux 2.6 / 3.x / 4.x (64-bit), 4 GB RAM, 32 GB disk.
2. Enable EFI in Settings → System → Motherboard.
3. Attach the ISO to the virtual optical drive.
4. Boot. Select "NovaiOS (live)" from the boot menu.

## Common build issues

| Symptom                                  | Fix                                       |
|------------------------------------------|-------------------------------------------|
| `error: rustc not found`                 | `rustup default stable`                    |
| `bindgen-cli: command not found`         | `cargo install --locked bindgen-cli`       |
| `CONFIG_RUST` not set after olddefconfig | ensure `rustc` is on PATH for `make`       |
| `pacstrap: command not found`            | run `build-iso.sh` inside archlinux container |
| `mksquashfs: out of memory`              | free up RAM or lower `-Xcompression-level` |
| ISO boots to dracut emergency shell      | check kernel cmdline `root=live:CDLABEL=...` matches `ISO_LABEL` env |
| `novai-init: no squashfs found`          | verify `filesystem.squashfs` exists in `/novai/live/` of the ISO |

For anything else, check the Actions log — the auto-fix bot may already
have a patch in flight.
