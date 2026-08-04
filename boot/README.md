# NovaiOS boot configuration

## Bootloader: systemd-boot (UEFI) + isolinux (BIOS)

The ISO is built with **both** boot paths so it works on any firmware:

- **UEFI**: `/EFI/BOOT/BOOTX64.EFI` is a unified kernel image (UKI)
  containing the kernel + initramfs + cmdline + os-release. The firmware
  loads it directly with no extra bootloader stage.
- **BIOS**: `/isolinux/isolinux.bin` + `ldlinux.c32` provide the legacy
  El Torito boot image.

## Kernel cmdline reference

All NovaiOS-specific cmdline keys are prefixed with `novai.`:

| Key                          | Default      | Meaning                                       |
|------------------------------|--------------|-----------------------------------------------|
| `novai.live=1`               | unset        | Live ISO boot — squashfs + overlay root       |
| `novai.root=<dev>:<fst>:<o>` | unset        | Real install boot from `<dev>`                |
| `novai.init=<path>`          | `/sbin/init` | Override the init binary to exec              |
| `novai.swap=<dev>`           | unset        | Enable this swap device early                 |
| `novai.squashfs=<path>`      | auto-detect  | Use this path for the squashfs                |
| `novai.debug=1`              | unset        | Enable debug logging in novai-init            |
| `novai.no_overlay=1`         | unset        | Skip the overlayfs (debug only)               |

Standard `init=` and `root=` are also honoured as fallbacks.

## Boot entries shipped on the ISO

The ISO's `/boot/loader/entries/` contains:

1. `novai.conf`           — Live mode (default).
2. `novai-installer.conf` — Install mode (root=/dev/sda2, ext4, rw).

## initramfs (dracut)

We ship a custom dracut module, `95novai`, that:

1. Mounts the ISO by `CDLABEL=NOVAI_ISO`.
2. Mounts `filesystem.squashfs` at `/run/novai/sqfs`.
3. Sets `NOVAI_SQUASHFS=/run/novai/sqfs` for novai-init.
4. Execs `/init` (which is our `novai-init` binary).

This means the kernel doesn't need a separate `initramfs=` cmdline —
the initramfs is bundled in the UKI / loaded by isolinux.

## Adding a custom boot entry

Drop a new file in `/boot/loader/entries/` with a unique name:

```
title   My custom entry
linux   /boot/vmlinuz-novai
initrd  /boot/initramfs-novai.img
options novai.live=1 novai.debug=1
```

`loader.conf`'s `default` key picks the entry by name (without `.conf`).
