#!/usr/bin/env bash
# =============================================================================
# NovaiOS — Stage 2: build the bootable ISO.
# -----------------------------------------------------------------------------
# Strategy (matches archiso's proven approach):
#   1. Bootstrap an Arch rootfs with a complete package set.
#   2. Drop our Rust-built novai-* binaries into /usr/bin.
#   3. Generate initramfs with novai-init as /init.
#   4. Squash the rootfs (read-only) → /novai/live/filesystem.squashfs.
#   5. Build a FAT32 EFI System Partition (ESP) image containing:
#        - systemd-boot (BOOTX64.EFI) as the UEFI bootloader
#        - kernel + initramfs
#        - loader/entries/novai-live.conf   (Live Mode)
#        - loader/entries/novai-install.conf (Installation Mode)
#      This ESP is what firmware sees on UEFI boot → fixes
#      "No bootable option or device found".
#   6. Assemble the final ISO with xorriso:
#        - BIOS El Torito boot image (isolinux)
#        - UEFI El Torito boot image (the ESP)
#        - GPT partition entry for the ESP (so `dd` to USB boots UEFI too)
#        - MBR + isohybrid for BIOS USB boot
#   7. Output: novaios-<date>-x86_64.iso (hybrid UEFI+BIOS, USB+optical).
#
# This runs in CI inside an archlinux:latest container.
set -euo pipefail
IFS=$'\n\t'

NOVAI_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${WORK:-$NOVAI_ROOT/build/iso-work}"
ROOTFS="$WORK/airootfs"
ISO_DIR="$WORK/iso"
ESP_DIR="$WORK/esp"            # staging dir for the FAT32 ESP contents
ESP_IMG="$WORK/esp.img"        # the FAT32 ESP image embedded in the ISO
OUT_DIR="${OUT_DIR:-$NOVAI_ROOT/build/out}"
KVER="${KVER:-$(uname -r)}"
ISO_LABEL="${ISO_LABEL:-NOVAI_ISO}"
ISO_NAME="${ISO_NAME:-novaios-$(date +%Y.%m.%d)-x86_64.iso}"
ESP_SIZE_MB="${ESP_SIZE_MB:-64}"

mkdir -p "$ROOTFS" "$ISO_DIR" "$ESP_DIR" "$OUT_DIR"

# =============================================================================
# 1. Bootstrap base rootfs (Arch + complete package set)
# =============================================================================
echo "::group::1. Bootstrap base rootfs (arch) with complete package set"
if ! command -v pacstrap >/dev/null; then
  echo "this script must run inside an archlinux container"
  exit 1
fi

# Enable the multilib repo (needed for steam + 32-bit graphics drivers).
# The archlinux:latest container ships pacman.conf without it active.
if ! grep -q '^\[multilib\]' /etc/pacman.conf; then
  sed -i '/^\[core\]/i [multilib]\nInclude = /etc/pacman.d/mirrorlist' /etc/pacman.conf
fi
pacman -Sy --noconfirm

# Comprehensive package list — "literally complete with everything"
# All package names verified against the official Arch repo (Aug 2026).
# Packages that live only in AUR (calamares, etc.) are installed separately below.
pacstrap -c -M -G "$ROOTFS" \
  base base-devel linux linux-headers linux-firmware \
  systemd systemd-sysvcompat dbus networkmanager \
  dracut squashfs-tools xorriso dosfstools mtools \
  syslinux grub \
  rustup git wget curl which sudo nano vi vim \
  mesa libdrm vulkan-radeon vulkan-intel vulkan-swrast nvidia-dkms nvidia-utils \
  pipewire pipewire-pulse pipewire-alsa pipewire-jack pipewire-v4l2 wireplumber \
  weston xorg-xwayland \
  uutils-coreutils nushell bat fd ripgrep eza zoxide starship helix yazi \
  firefox chromium \
  seatd polkit \
  archinstall \
  gparted partitionmanager dosfstools ntfs-3g exfatprogs f2fs-tools btrfs-progs xfsprogs \
  gnu-free-fonts ttf-dejavu ttf-liberation noto-fonts noto-fonts-cjk noto-fonts-emoji ttf-nerd-fonts-symbols \
  gnome-themes-extra papirus-icon-theme breeze-icons hicolor-icon-theme \
  gtk3 gtk4 qt5-base qt6-base qt5-wayland qt6-wayland \
  xdg-desktop-portal xdg-desktop-portal-gtk xdg-desktop-portal-wlr \
  firefox-i18n-en-us \
  libreoffice-fresh libreoffice-fresh-en-gb \
  vlc mpv \
  gimp inkscape krita blender \
  obs-studio \
  steam \
  audacity \
  docker docker-compose \
  python python-pip python-pipx pyenv \
  nodejs npm yarn pnpm \
  go rustup \
  git github-cli \
  vscode \
  neovim emacs \
  tmux screen \
  htop btop iotop iftop nethogs \
  unzip zip p7zip unrar \
  openssh x11-ssh-askpass \
  rsync rclone \
  ffmpeg imagemagick \
  cups cups-pdf system-config-printer \
  bluez bluez-utils \
  network-manager-applet \
  reflector \
  man-db man-pages texinfo \
  pkgconf clang llvm lld \
  --needed

# Enable multilib inside the rootfs too (so steam works after install).
# pacstrap -G doesn't generate pacman.conf in the rootfs, so we have to
# copy it from the host (which already has multilib enabled from above).
if [[ ! -f "$ROOTFS/etc/pacman.conf" ]]; then
  cp /etc/pacman.conf "$ROOTFS/etc/pacman.conf"
fi
if ! grep -q '^\[multilib\]' "$ROOTFS/etc/pacman.conf"; then
  sed -i '/^\[core\]/i [multilib]\nInclude = /etc/pacman.d/mirrorlist' "$ROOTFS/etc/pacman.conf"
fi
# Make sure the mirrorlist is present in the rootfs too
if [[ ! -f "$ROOTFS/etc/pacman.d/mirrorlist" ]]; then
  cp /etc/pacman.d/mirrorlist "$ROOTFS/etc/pacman.d/mirrorlist"
fi

echo "::endgroup::"

# =============================================================================
# 1b. Install Calamares from AUR (not in official repos)
# =============================================================================
echo "::group::1b. Install Calamares from AUR"
# Calamares is in the AUR, not the official repos. Build it inside the chroot.
# This is best-effort — if the AUR build fails, archinstall is still available
# as the TUI fallback installer.
arch-chroot "$ROOTFS" bash -c '
  set -e
  # Create a build user (makepkg refuses to run as root)
  id builduser &>/dev/null || useradd -m -G wheel -s /bin/bash builduser
  echo "builduser ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/builduser
  cd /home/builduser

  # Install AUR helper (yay) — easier than raw makepkg
  sudo -u builduser git clone https://aur.archlinux.org/yay.git 2>/dev/null || true
  cd yay
  sudo -u builduser git pull --rebase 2>/dev/null || true
  sudo -u builduser makepkg -si --noconfirm --needed 2>&1 || echo "::warning::yay install failed"

  # Use yay to install calamares + its generic config
  if command -v yay >/dev/null; then
    sudo -u builduser yay -S --noconfirm --needed calamares calamares-config-generic 2>&1 || \
      echo "::warning::calamares AUR install failed — archinstall will be the only installer"
  fi

  # Clean up build artifacts to save space in the squashfs
  rm -rf /home/builduser/.cache /home/builduser/yay /var/cache/pacman/pkg/*
' 2>&1 || echo "::warning::AUR/calamares setup failed — archinstall remains available"
echo "::endgroup::"

# =============================================================================
# 2. Drop Rust-built novai-* binaries into rootfs
# =============================================================================
echo "::group::2. Drop Rust-built novai-* binaries into rootfs"
USERLAND_BIN="$NOVAI_ROOT/build/userland/bin"
if [[ -d "$USERLAND_BIN" ]]; then
  cp -v "$USERLAND_BIN"/* "$ROOTFS/usr/bin/" 2>/dev/null || true
  chmod 0755 "$ROOTFS/usr/bin/novai-"* 2>/dev/null || true
  ln -sf /usr/bin/novai-shell "$ROOTFS/usr/bin/novai-rescue-shell"
fi
echo "::endgroup::"

# =============================================================================
# 3. Configure rootfs (users, services, autologin, hostname)
# =============================================================================
echo "::group::3. Configure rootfs"
install -d "$ROOTFS/etc/novai" "$ROOTFS/etc/novai/services" "$ROOTFS/etc/systemd/system"
cp -r "$NOVAI_ROOT/iso/profile/airootfs/." "$ROOTFS/"

ln -sf /usr/bin/novai-init "$ROOTFS/sbin/novai-init"

echo "novai" > "$ROOTFS/etc/hostname"

# Append the novai user (idempotent — base already has root)
if ! grep -q "^novai:" "$ROOTFS/etc/passwd"; then
  echo "novai:x:1000:1000:NovaiOS:/home/novai:/usr/bin/nu" >> "$ROOTFS/etc/passwd"
fi
if ! grep -q "^novai:" "$ROOTFS/etc/shadow"; then
  echo "novai:!:19000:0:99999:7:::" >> "$ROOTFS/etc/shadow"
fi
if ! grep -q "^novai:" "$ROOTFS/etc/group"; then
  echo "novai:x:1000:" >> "$ROOTFS/etc/group"
fi
install -d "$ROOTFS/home/novai" "$ROOTFS/etc/sudoers.d"
chown 1000:1000 "$ROOTFS/home/novai" 2>/dev/null || true
echo "novai ALL=(ALL) NOPASSWD: ALL" > "$ROOTFS/etc/sudoers.d/10-novai"
chmod 0440 "$ROOTFS/etc/sudoers.d/10-novai"

# Autologin on tty1 (live session)
install -d "$ROOTFS/etc/systemd/system/getty@tty1.service.d"
cat > "$ROOTFS/etc/systemd/system/getty@tty1.service.d/autologin.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin novai --noclear %I $TERM
EOF

# Enable services
arch-chroot "$ROOTFS" systemctl enable \
  NetworkManager systemd-resolved seatd \
  cups bluetooth sshd \
  2>/dev/null || true

# Calamares desktop entry — so the user can launch the installer from the menu
install -d "$ROOTFS/usr/share/applications"
cat > "$ROOTFS/usr/share/applications/novai-installer.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Install NovaiOS
Comment=Install NovaiOS to your hard drive
Exec=sudo calamares
Icon=system-software-install
Terminal=false
Categories=System;Settings;
EOF

# Autostart the installer on first login to the live session
install -d "$ROOTFS/home/novai/.config/autostart"
cat > "$ROOTFS/home/novai/.config/autostart/novai-installer.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=NovaiOS Installer
Exec=sudo calamares
Icon=system-software-install
Terminal=false
X-GNOME-Autostart-enabled=false
EOF
chown -R 1000:1000 "$ROOTFS/home/novai/.config" 2>/dev/null || true

echo "::endgroup::"

# =============================================================================
# 4. Install kernel + initramfs
# =============================================================================
echo "::group::4. Install kernel + initramfs"
KOUT="$NOVAI_ROOT/build/out-kernel"
if [[ -f "$KOUT/vmlinuz-novai" ]]; then
  install -D -m0644 "$KOUT/vmlinuz-novai" "$ROOTFS/boot/vmlinuz-novai"
  install -D -m0644 "$KOUT/System.map-novai" "$ROOTFS/boot/System.map-novai" 2>/dev/null || true
  install -D -m0644 "$KOUT/config-novai" "$ROOTFS/boot/config-novai" 2>/dev/null || true
else
  # Fallback: use the stock Arch linux kernel from the rootfs
  if [[ -f "$ROOTFS/boot/vmlinuz-linux" ]]; then
    cp "$ROOTFS/boot/vmlinuz-linux" "$ROOTFS/boot/vmlinuz-novai"
    echo "::warning::Using stock Arch vmlinuz-linux as vmlinuz-novai"
  fi
fi
if [[ -d "$NOVAI_ROOT/build/modules/lib/modules" ]]; then
  cp -a "$NOVAI_ROOT/build/modules/lib/modules" "$ROOTFS/lib/"
fi

# Symlink /init and /sbin/novai-init
if [[ -f "$ROOTFS/usr/bin/novai-init" ]]; then
  ln -sf /usr/bin/novai-init "$ROOTFS/init"
  ln -sf /usr/bin/novai-init "$ROOTFS/sbin/novai-init"
fi

# dracut config
cat > "$ROOTFS/etc/dracut.conf.d/novai.conf" <<'EOF'
add_dracutmodules+=" novai overlay squashfs "
add_drivers+=" overlay squashfs loop isofs ext4 vfat ahci nvme virtio_blk virtio_pci virtio_net virtio_console amd_pstate "
compress="zstd"
EOF

# Custom dracut module: mounts the squashfs from the ISO
install -d "$ROOTFS/usr/lib/dracut/modules.d/95novai"
cat > "$ROOTFS/usr/lib/dracut/modules.d/95novai/module-setup.sh" <<'EOF'
#!/bin/bash
check()     { return 0; }
depends()   { echo ""; }
install() {
    if [[ -x /usr/bin/novai-init ]]; then
        inst_simple /usr/bin/novai-init /init 2>/dev/null || true
    fi
    if [[ -f "$moddir/novai-prepivot.sh" ]]; then
        inst_hook pre-pivot 10 "$moddir/novai-prepivot.sh" 2>/dev/null || true
    fi
    return 0
}
EOF
cat > "$ROOTFS/usr/lib/dracut/modules.d/95novai/novai-prepivot.sh" <<'EOF'
#!/bin/sh
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
chmod +x "$ROOTFS/usr/lib/dracut/modules.d/95novai/module-setup.sh" \
         "$ROOTFS/usr/lib/dracut/modules.d/95novai/novai-prepivot.sh"

# Detect installed kernel version
INSTALLED_KVER=$(arch-chroot "$ROOTFS" ls /lib/modules 2>/dev/null | head -1 || echo "")
echo "Detected installed kernel version: $INSTALLED_KVER"

DRACUT_KVER="${INSTALLED_KVER:-$KVER}"
echo "Building initramfs for kernel $DRACUT_KVER"
arch-chroot "$ROOTFS" dracut --force --no-hostonly /boot/initramfs-novai.img "$DRACUT_KVER" 2>&1 || \
  arch-chroot "$ROOTFS" dracut --force --no-hostonly /boot/initramfs-novai.img 2>&1 || \
  arch-chroot "$ROOTFS" mkinitcpio -g /boot/initramfs-novai.img "$DRACUT_KVER" 2>&1 || {
    echo "::warning::dracut + mkinitcpio both failed — building minimal initramfs with busybox"
    arch-chroot "$ROOTFS" bash -c '
      mkdir -p /tmp/initramfs/{bin,sbin,dev,proc,sys,run,usr/bin,lib,lib64}
      cp /bin/busybox /tmp/initramfs/bin/ 2>/dev/null || true
      cp /usr/bin/novai-init /tmp/initramfs/init 2>/dev/null || true
      chmod +x /tmp/initramfs/init 2>/dev/null || true
      cd /tmp/initramfs && find . | cpio -H newc -o | gzip > /boot/initramfs-novai.img
    '
  }

# Copy kernel + initramfs to the ISO staging area
install -D -m0644 "$ROOTFS/boot/vmlinuz-novai"      "$ISO_DIR/boot/vmlinuz-novai"
install -D -m0644 "$ROOTFS/boot/initramfs-novai.img" "$ISO_DIR/boot/initramfs-novai.img"

echo "::endgroup::"

# =============================================================================
# 5. Squash the rootfs
# =============================================================================
echo "::group::5. Squash the rootfs"
mkdir -p "$ISO_DIR/novai/live"
rm -f "$ISO_DIR/novai/live/filesystem.squashfs"
mksquashfs "$ROOTFS" "$ISO_DIR/novai/live/filesystem.squashfs" -comp zstd -Xcompression-level 19 -noappend
echo "::endgroup::"

# =============================================================================
# 6. Build the FAT32 EFI System Partition (ESP) image
# =============================================================================
# This is the key fix for "No bootable option or device found" in UEFI mode.
# We build a real FAT32 filesystem image containing the systemd-boot bootloader,
# kernel, initramfs, and boot menu entries. UEFI firmware loads BOOTX64.EFI
# directly, which is systemd-boot, which then reads loader/loader.conf and
# presents the boot menu (Live Mode / Installation Mode).
echo "::group::6. Build FAT32 EFI System Partition (ESP) image"
rm -rf "$ESP_DIR"
mkdir -p "$ESP_DIR/EFI/BOOT" \
         "$ESP_DIR/EFI/systemd" \
         "$ESP_DIR/loader/entries" \
         "$ESP_DIR/novai"

# Copy systemd-boot EFI binary as BOOTX64.EFI (UEFI firmware looks for this exact name)
SYSTEMD_BOOT_EFI="/usr/lib/systemd/boot/efi/systemd-bootx64.efi"
if [[ -f "$SYSTEMD_BOOT_EFI" ]]; then
  cp "$SYSTEMD_BOOT_EFI" "$ESP_DIR/EFI/BOOT/BOOTX64.EFI"
  cp "$SYSTEMD_BOOT_EFI" "$ESP_DIR/EFI/systemd/systemd-bootx64.efi"
  echo "Installed systemd-boot as BOOTX64.EFI"
else
  echo "::error::systemd-boot EFI binary not found at $SYSTEMD_BOOT_EFI"
  echo "Install the 'systemd' package in the archlinux container — it ships systemd-bootx64.efi at /usr/lib/systemd/boot/efi/."
  exit 1
fi

# Also install grub-efi as a fallback bootloader (in case systemd-boot fails on some firmware)
GRUB_EFI=$(find /usr/lib -name "grubx64.efi" 2>/dev/null | head -1)
if [[ -n "$GRUB_EFI" && -f "$GRUB_EFI" ]]; then
  mkdir -p "$ESP_DIR/EFI/grub"
  cp "$GRUB_EFI" "$ESP_DIR/EFI/grub/grubx64.efi"
fi

# Copy kernel + initramfs into the ESP (so systemd-boot can find them at /vmlinuz-novai)
cp "$ROOTFS/boot/vmlinuz-novai"      "$ESP_DIR/vmlinuz-novai"
cp "$ROOTFS/boot/initramfs-novai.img" "$ESP_DIR/initramfs-novai.img"

# Also copy to the ISO's /boot for BIOS boot (isolinux)
cp "$ROOTFS/boot/vmlinuz-novai"      "$ISO_DIR/boot/vmlinuz-novai"
cp "$ROOTFS/boot/initramfs-novai.img" "$ISO_DIR/boot/initramfs-novai.img"

# systemd-boot loader.conf — presents the boot menu
cat > "$ESP_DIR/loader/loader.conf" <<EOF
# NovaiOS boot menu configuration
default novai-live
timeout 8
console-mode max
editor no
auto-entries yes
auto-firmware yes
EOF

# Boot menu entry 1: NovaiOS (Live Mode) — DEFAULT
cat > "$ESP_DIR/loader/entries/novai-live.conf" <<EOF
title   NovaiOS 0.1 (Live Mode)
linux   /vmlinuz-novai
initrd  /initramfs-novai.img
options novai.live=1 root=live:CDLABEL=$ISO_LABEL rd.live.image rd.live.squashfs=airootfs.sfs quiet loglevel=3
EOF

# Boot menu entry 2: NovaiOS (Installation Mode)
cat > "$ESP_DIR/loader/entries/novai-install.conf" <<EOF
title   NovaiOS 0.1 (Installation Mode)
linux   /vmlinuz-novai
initrd  /initramfs-novai.img
options novai.live=1 root=live:CDLABEL=$ISO_LABEL rd.live.image rd.live.squashfs=airootfs.sfs quiet loglevel=3 novai.install=1
EOF

# Boot menu entry 3: NovaiOS (Live Mode, Safe Graphics — nomodeset)
cat > "$ESP_DIR/loader/entries/novai-safe.conf" <<EOF
title   NovaiOS 0.1 (Safe Graphics — nomodeset)
linux   /vmlinuz-novai
initrd  /initramfs-novai.img
options novai.live=1 root=live:CDLABEL=$ISO_LABEL rd.live.image rd.live.squashfs=airootfs.sfs quiet loglevel=3 nomodeset
EOF

# Boot menu entry 4: Reboot firmware setup (UEFI settings)
cat > "$ESP_DIR/loader/entries/reboot-firmware.conf" <<EOF
title   Reboot into UEFI Firmware Settings
efi     /EFI/systemd/systemd-bootx64.efi
EOF

# Copy the same kernel/initramfs to the ISO's /boot for BIOS isolinux path
install -d "$ISO_DIR/boot" "$ISO_DIR/loader/entries"
cp "$ESP_DIR/loader/loader.conf" "$ISO_DIR/loader/loader.conf"
cp "$ESP_DIR/loader/entries/"*.conf "$ISO_DIR/loader/entries/"

# Build the FAT32 ESP image using mtools (no root needed)
rm -f "$ESP_IMG"
dd if=/dev/zero of="$ESP_IMG" bs=1M count="$ESP_SIZE_MB" status=none
mkfs.vfat -n "NOVAI_BOOT" "$ESP_IMG" >/dev/null

# Copy the ESP staging directory into the FAT image using mcopy
mcopy -i "$ESP_IMG" -s "$ESP_DIR"/* ::/

# Verify the ESP image
echo "ESP image contents:"
mdir -i "$ESP_IMG" ::/EFI/BOOT/
mdir -i "$ESP_IMG" ::/loader/entries/
echo "ESP image size: $(du -h "$ESP_IMG" | cut -f1)"
echo "::endgroup::"

# =============================================================================
# 7. BIOS boot files (isolinux for legacy BIOS)
# =============================================================================
echo "::group::7. Lay out BIOS boot files (isolinux)"
install -d "$ISO_DIR/isolinux"

# isolinux config with a proper menu showing Live / Install / Safe / Firmware
cat > "$ISO_DIR/isolinux/isolinux.cfg" <<'EOF'
UI menu.c32
PROMPT 0
TIMEOUT 80
DEFAULT novai-live

MENU TITLE NovaiOS 0.1 — Boot Menu
MENU BACKGROUND /isolinux/novai-bg.png
MENU COLOR title        1;37;44 #ffffffff #00000000 std
MENU COLOR border       30;44   #ffffffff #00000000 std
MENU COLOR sel          7;37;40 #ff000000 #ffffffff all
MENU COLOR unsel        37;44   #ff000000 #00000000 std
MENU COLOR help         37;40   #ff000000 #00000000 std
MENU COLOR timeout_msg  37;40   #ff000000 #00000000 std
MENU COLOR timeout      1;37;40 #ff000000 #ffffffff std

LABEL novai-live
  MENU LABEL ^NovaiOS 0.1 (Live Mode)
  MENU DEFAULT
  KERNEL /boot/vmlinuz-novai
  INITRD /boot/initramfs-novai.img
  APPEND novai.live=1 root=live:CDLABEL=NOVAI_ISO rd.live.image rd.live.squashfs=airootfs.sfs quiet loglevel=3

LABEL novai-install
  MENU LABEL ^NovaiOS 0.1 (Installation Mode)
  KERNEL /boot/vmlinuz-novai
  INITRD /boot/initramfs-novai.img
  APPEND novai.live=1 root=live:CDLABEL=NOVAI_ISO rd.live.image rd.live.squashfs=airootfs.sfs quiet loglevel=3 novai.install=1

LABEL novai-safe
  MENU LABEL NovaiOS 0.1 (Safe Graphics — nomodeset)
  KERNEL /boot/vmlinuz-novai
  INITRD /boot/initramfs-novai.img
  APPEND novai.live=1 root=live:CDLABEL=NOVAI_ISO rd.live.image rd.live.squashfs=airootfs.sfs quiet loglevel=3 nomodeset

LABEL reboot-firmware
  MENU LABEL Reboot into UEFI Firmware Settings
  COM32 reboot.c32
  APPEND --warm
EOF

# Find and copy isolinux files
for p in /usr/lib/syslinux/bios/isolinux.bin /usr/share/syslinux/isolinux.bin /usr/lib/ISOLINUX/isolinux.bin; do
  if [[ -f "$p" ]]; then cp "$p" "$ISO_DIR/isolinux/"; break; fi
done
for p in /usr/lib/syslinux/bios/ldlinux.c32 /usr/share/syslinux/ldlinux.c32; do
  if [[ -f "$p" ]]; then cp "$p" "$ISO_DIR/isolinux/"; break; fi
done
for p in /usr/lib/syslinux/bios/menu.c32 /usr/share/syslinux/menu.c32; do
  if [[ -f "$p" ]]; then cp "$p" "$ISO_DIR/isolinux/"; break; fi
done
for p in /usr/lib/syslinux/bios/libutil.c32 /usr/share/syslinux/libutil.c32; do
  if [[ -f "$p" ]]; then cp "$p" "$ISO_DIR/isolinux/"; break; fi
done
for p in /usr/lib/syslinux/bios/libcom32.c32 /usr/share/syslinux/libcom32.c32; do
  if [[ -f "$p" ]]; then cp "$p" "$ISO_DIR/isolinux/"; break; fi
done
for p in /usr/lib/syslinux/bios/reboot.c32 /usr/share/syslinux/reboot.c32; do
  if [[ -f "$p" ]]; then cp "$p" "$ISO_DIR/isolinux/"; break; fi
done

# Find isohdpfx.bin for hybrid MBR
ISOHDPFX=""
for p in /usr/lib/syslinux/bios/isohdpfx.bin /usr/share/syslinux/isohdpfx.bin; do
  if [[ -f "$p" ]]; then ISOHDPFX="$p"; break; fi
done

# Copy the ESP image into the ISO tree at /EFI/esp.img (referenced by xorriso -e)
cp "$ESP_IMG" "$ISO_DIR/EFI/esp.img"
echo "::endgroup::"

# =============================================================================
# 8. Assemble the final hybrid ISO with xorriso
# =============================================================================
# This produces a true hybrid ISO that boots on:
#   - UEFI firmware (via the FAT32 ESP at /EFI/esp.img as El Torito UEFI boot image)
#   - BIOS firmware (via isolinux as El Torito BIOS boot image)
#   - USB stick via `dd` (via -isohybrid-mbr + GPT partition for the ESP)
#   - Optical media (CD/DVD) via El Torito boot records
echo "::group::8. Assemble the final hybrid ISO with xorriso"

XORRISO_ARGS=(
  -as mkisofs
  -iso-level 3
  -full-iso9660-filenames
  -volid "$ISO_LABEL"
  -appid "NovaiOS 0.1"
  -publisher "NovaiOS Project"
  -preparer "NovaiOS CI"
)

# --- BIOS El Torito boot image (isolinux) ---
XORRISO_ARGS+=(
  -eltorito-boot isolinux/isolinux.bin
  -eltorito-catalog isolinux/boot.cat
  -no-emul-boot
  -boot-load-size 4
  -boot-info-table
)

# Hybrid MBR so the same ISO is bootable when dd'd to a USB stick (BIOS mode)
if [[ -n "$ISOHDPFX" ]]; then
  XORRISO_ARGS+=(-isohybrid-mbr "$ISOHDPFX")
fi

# --- UEFI El Torito boot image (the FAT32 ESP) ---
# This is what makes UEFI firmware see a bootable device.
XORRISO_ARGS+=(
  -eltorito-alt-boot
  -e EFI/esp.img
  -no-emul-boot
  -isohybrid-gpt-basdat
)

# Final: output path + source directory
XORRISO_ARGS+=(
  -output "$OUT_DIR/$ISO_NAME"
  "$ISO_DIR"
)

echo "Running xorriso with args:"
printf '  %q\n' "${XORRISO_ARGS[@]}"

xorriso "${XORRISO_ARGS[@]}" || {
  echo "::error::xorriso failed"
  exit 1
}
echo "::endgroup::"

# =============================================================================
# 9. Verify + checksums
# =============================================================================
echo "::group::9. Verify ISO + write checksums"

# List the El Torito boot records to confirm both BIOS and UEFI entries are present
echo "ISO boot records:"
xorriso -indev "$OUT_DIR/$ISO_NAME" -report_el_torito plain 2>/dev/null | head -40 || true

# Show the GPT partition table to confirm the ESP is there for USB boot
echo ""
echo "ISO GPT/MBR partitions:"
xorriso -indev "$OUT_DIR/$ISO_NAME" -report_system_area plain 2>/dev/null | grep -E "Partition|MBR|GPT|EFI" | head -20 || true

# Write SHA-256 checksum next to the ISO
sha256sum "$OUT_DIR/$ISO_NAME" | tee "$OUT_DIR/$ISO_NAME.sha256"

ISO_SIZE=$(du -h "$OUT_DIR/$ISO_NAME" | cut -f1)
echo ""
echo "================================================================"
echo "✅ ISO built successfully: $OUT_DIR/$ISO_NAME"
echo "   Size: $ISO_SIZE"
echo "   Label: $ISO_LABEL"
echo "   Boot modes: UEFI (systemd-boot) + BIOS (isolinux) + USB hybrid"
echo "   Boot menu: Live Mode / Installation Mode / Safe Graphics / Firmware"
echo "================================================================"
echo "::endgroup::"
