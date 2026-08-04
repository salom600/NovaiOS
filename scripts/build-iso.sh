#!/usr/bin/env bash
# =============================================================================
# NovaiOS — Stage 2: build the bootable ISO.
# -----------------------------------------------------------------------------
# Strategy:
#   1. Start from an Arch rootfs (pacman -S base + Rust userland)
#   2. Drop our Rust-built binaries into /usr/bin
#   3. Generate dracut initramfs with novai-init as /init
#   4. Squash the rootfs (read-only) → /iso/airootfs.sfs
#   5. xorriso assembles the final ISO with systemd-boot + eltorito boot img.
#
# This runs in CI inside an archlinux:latest container.
set -euo pipefail
IFS=$'\n\t'

NOVAI_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${WORK:-$NOVAI_ROOT/build/iso-work}"
ROOTFS="$WORK/airootfs"
ISO_DIR="$WORK/iso"
OUT_DIR="${OUT_DIR:-$NOVAI_ROOT/build/out}"
KVER="${KVER:-$(uname -r)}"          # default to the running kernel (works in archlinux container)
ISO_LABEL="${ISO_LABEL:-NOVAI_ISO}"
ISO_NAME="${ISO_NAME:-novaios-$(date +%Y.%m.%d)-x86_64.iso}"

mkdir -p "$ROOTFS" "$ISO_DIR" "$OUT_DIR"

echo "::group::1. Bootstrap base rootfs (arch)"
if ! command -v pacstrap >/dev/null; then
  echo "this script must run inside an archlinux container"
  exit 1
fi
pacstrap -c -M -G "$ROOTFS" \
  base base-devel linux-firmware \
  systemd dbus networkmanager \
  dracut squashfs-tools xorriso dosfstools mtools \
  rustup git wget curl which sudo nano vi \
  mesa libdrm vulkan-radeon vulkan-intel nvidia-dkms nvidia-utils \
  pipewire pipewire-pulse pipewire-alsa pipewire-jack wireplumber \
  weston xorg-xwayland \
  uutils-coreutils nushell bat fd ripgrep eza zoxide starship helix yazi \
  firefox chromium \
  seatd polkit

echo "::endgroup::"

echo "::group::2. Drop Rust-built novai-* binaries into rootfs"
USERLAND_BIN="$NOVAI_ROOT/build/userland/bin"
if [[ -d "$USERLAND_BIN" ]]; then
  cp -v "$USERLAND_BIN"/* "$ROOTFS/usr/bin/"
  chmod 0755 "$ROOTFS/usr/bin/novai-"*
  # Symlink novai-shell as a fallback /bin/sh alternative
  ln -sf /usr/bin/novai-shell "$ROOTFS/usr/bin/novai-rescue-shell"
fi
echo "::endgroup::"

echo "::group::3. Configure rootfs"
install -d "$ROOTFS/etc/novai" "$ROOTFS/etc/novai/services" "$ROOTFS/etc/systemd/system"
cp -r "$NOVAI_ROOT/iso/profile/airootfs/." "$ROOTFS/"

# Make novai-init the kernel's init= target on installed boots.
ln -sf /usr/bin/novai-init "$ROOTFS/sbin/novai-init"

# Default hostname
echo "novai" > "$ROOTFS/etc/hostname"

# Default user (no password on live ISO; sudo NOPASSWD)
echo "root:x:0:0:root:/root:/usr/bin/novai-shell" >> "$ROOTFS/etc/passwd"
echo "novai:x:1000:1000:NovaiOS:/home/novai:/usr/bin/nu" >> "$ROOTFS/etc/passwd"
echo "novai:!:19000:0:99999:7:::" >> "$ROOTFS/etc/shadow"
install -d "$ROOTFS/home/novai" "$ROOTFS/etc/sudoers.d"
echo "novai ALL=(ALL) NOPASSWD: ALL" > "$ROOTFS/etc/sudoers.d/10-novai"

# Autologin on tty1
install -d "$ROOTFS/etc/systemd/system/getty@tty1.service.d"
cat > "$ROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin novai --noclear %I $TERM
EOF

# Enable services
arch-chroot "$ROOTFS" systemctl enable NetworkManager systemd-resolved seatd || true

# Boot loader config
install -d "$ROOTFS/boot/loader/entries"
cat > "$ROOTFS/boot/loader/loader.conf" <<'EOF'
default novai
timeout 5
console-mode max
editor no
EOF

cat > "$ROOTFS/boot/loader/entries/novai.conf" <<EOF
title   NovaiOS (live)
linux   /boot/vmlinuz-novai
initrd  /boot/initramfs-novai.img
options novai.live=1 root=live:CDLABEL=$ISO_LABEL rd.live.image rd.live.squashfs=airootfs.sfs quiet loglevel=3
EOF

cat > "$ROOTFS/boot/loader/entries/novai-installer.conf" <<EOF
title   NovaiOS (install)
linux   /boot/vmlinuz-novai
initrd  /boot/initramfs-novai.img
options novai.root=/dev/sda2:ext4:rw novai.init=/usr/bin/novai-init quiet loglevel=3
EOF

echo "::endgroup::"

echo "::group::4. Install our kernel + initramfs into rootfs"
KOUT="$NOVAI_ROOT/build/out-kernel"
if [[ -f "$KOUT/vmlinuz-novai" ]]; then
  install -D -m0644 "$KOUT/vmlinuz-novai" "$ROOTFS/boot/vmlinuz-novai"
  install -D -m0644 "$KOUT/System.map-novai" "$ROOTFS/boot/System.map-novai" 2>/dev/null || true
  install -D -m0644 "$KOUT/config-novai" "$ROOTFS/boot/config-novai" 2>/dev/null || true
fi
if [[ -d "$NOVAI_ROOT/build/modules/lib/modules" ]]; then
  cp -a "$NOVAI_ROOT/build/modules/lib/modules" "$ROOTFS/lib/"
fi

# dracut initramfs
cat > "$ROOTFS/etc/dracut.conf.d/novai.conf" <<'EOF'
add_dracutmodules+=" novai overlay squashfs "
add_drivers+=" overlay squashfs loop isofs ext4 vfat ahci nvme virtio_blk virtio_pci amd_pstate "
compress="zstd"
EOF
install -d "$ROOTFS/usr/lib/dracut/modules.d/95novai"
cat > "$ROOTFS/usr/lib/dracut/modules.d/95novai/module-setup.sh" <<'EOF'
#!/bin/bash
check() { return 0; }
depends() { echo overlay squashfs; }
install() {
    inst_simple /usr/bin/novai-init /init
    inst_hook pre-pivot 10 "$moddir/novai-prepivot.sh"
}
EOF
cat > "$ROOTFS/usr/lib/dracut/modules.d/95novai/novai-prepivot.sh" <<'EOF'
#!/bin/sh
# mount the squashfs from the ISO into /run/novai/sqfs and let novai-init overlay it
set -e
. /lib/dracut-lib.sh
LABEL=$(getarg root= | sed 's/live:CDLABEL=//')
if [ -n "$LABEL" ]; then
  mkdir -p /run/novai/iso /run/novai/sqfs
  mount -t iso9660 -o ro LABEL=$LABEL /run/novai/iso || \
    mount -t iso9660 -o ro /dev/disk/by-label/$LABEL /run/novai/iso
  mount -t squashfs -o ro /run/novai/iso/novai/live/filesystem.squashfs /run/novai/sqfs
  export NOVAI_SQUASHFS=/run/novai/sqfs
fi
EOF
chmod +x "$ROOTFS/usr/lib/dracut/modules.d/95novai/module-setup.sh" "$ROOTFS/usr/lib/dracut/modules.d/95novai/novai-prepivot.sh"

arch-chroot "$ROOTFS" dracut --force --no-hostonly /boot/initramfs-novai.img "$KVER-novai" || \
  arch-chroot "$ROOTFS" dracut --force --no-hostonly /boot/initramfs-novai.img

echo "::endgroup::"

echo "::group::5. Squash the rootfs"
mkdir -p "$ISO_DIR/novai/live"
rm -f "$ISO_DIR/novai/live/filesystem.squashfs"
mksquashfs "$ROOTFS" "$ISO_DIR/novai/live/filesystem.squashfs" -comp zstd -Xcompression-level 19 -noappend
echo "::endgroup::"

echo "::group::6. Lay out the ISO tree (UEFI + BIOS)"
install -d "$ISO_DIR/boot" "$ISO_DIR/EFI/BOOT" "$ISO_DIR/loader/entries"
cp "$ROOTFS/boot/vmlinuz-novai"      "$ISO_DIR/boot/vmlinuz-novai"
cp "$ROOTFS/boot/initramfs-novai.img" "$ISO_DIR/boot/initramfs-novai.img"
cp "$ROOTFS/boot/loader/loader.conf"  "$ISO_DIR/loader/loader.conf"
cp "$ROOTFS/boot/loader/entries/"*.conf "$ISO_DIR/loader/entries/"

# UEFI: build a standalone systemd-boot UKI stub
EFISTUB=$(find /usr/lib -name linuxx64.efi.stub 2>/dev/null | head -1)
if [[ -n "$EFISTUB" && -f "$EFISTUB" ]]; then
  objcopy \
    --add-section .osrel="$ROOTFS/usr/lib/os-release"   --change-section-vma .osrel=0x20000 \
    --add-section .cmdline=/tmp/novai-cmdline.txt       --change-section-vma .cmdline=0x30000 \
    --add-section .linux="$ISO_DIR/boot/vmlinuz-novai"  --change-section-vma .linux=0x40000 \
    --add-section .initrd="$ISO_DIR/boot/initramfs-novai.img" --change-section-vma .initrd=0x3000000 \
    "$EFISTUB" "$ISO_DIR/EFI/BOOT/BOOTX64.EFI"
  printf "novai.live=1 root=live:CDLABEL=$ISO_LABEL rd.live.image rd.live.squashfs=airootfs.sfs quiet" > /tmp/novai-cmdline.txt
  objcopy \
    --add-section .osrel="$ROOTFS/usr/lib/os-release"   --change-section-vma .osrel=0x20000 \
    --add-section .cmdline=/tmp/novai-cmdline.txt       --change-section-vma .cmdline=0x30000 \
    --add-section .linux="$ISO_DIR/boot/vmlinuz-novai"  --change-section-vma .linux=0x40000 \
    --add-section .initrd="$ISO_DIR/boot/initramfs-novai.img" --change-section-vma .initrd=0x3000000 \
    "$EFISTUB" "$ISO_DIR/EFI/BOOT/BOOTX64.EFI"
else
  echo "::warning::EFI stub not found — skipping UEFI boot image"
fi

# BIOS: eltorito boot image via isolinux
install -d "$ISO_DIR/isolinux"
cat > "$ISO_DIR/isolinux/isolinux.cfg" <<'EOF'
DEFAULT novai
PROMPT 0
TIMEOUT 5
LABEL novai
  LINUX /boot/vmlinuz-novai
  INITRD /boot/initramfs-novai.img
  APPEND novai.live=1 root=live:CDLABEL=NOVAI_ISO rd.live.image rd.live.squashfs=airootfs.sfs quiet
EOF
cp /usr/lib/syslinux/bios/isolinux.bin   "$ISO_DIR/isolinux/" 2>/dev/null || true
cp /usr/lib/syslinux/bios/ldlinux.c32     "$ISO_DIR/isolinux/" 2>/dev/null || true
cp /usr/lib/syslinux/bios/menu.c32        "$ISO_DIR/isolinux/" 2>/dev/null || true
echo "::endgroup::"

echo "::group::7. Assemble the ISO with xorriso"
xorriso -as mkisofs \
  -iso-level 3 \
  -full-iso9660-filenames \
  -volid "$ISO_LABEL" \
  -eltorito-boot isolinux/isolinux.bin \
  -eltorito-catalog isolinux/boot.cat \
  -no-emul-boot -boot-load-size 4 -boot-info-table \
  -isohybrid-mbr /usr/lib/syslinux/bios/isohdpfx.bin \
  -eltorito-alt-boot \
  -e EFI/BOOT/BOOTX64.EFI \
  -no-emul-boot -isohybrid-gpt-basdat \
  -output "$OUT_DIR/$ISO_NAME" \
  "$ISO_DIR"
echo "::endgroup::"

sha256sum "$OUT_DIR/$ISO_NAME" | tee "$OUT_DIR/$ISO_NAME.sha256"
echo "✅ ISO built: $OUT_DIR/$ISO_NAME"
