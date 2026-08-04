# NovaiOS Release Notes

## v0.1.0 — First bootable ISO

This is the very first NovaiOS build. It is intended for testing in VMs
and **should not be installed on production hardware yet**.

### What works

- Boot the ISO in BIOS or UEFI mode.
- Live session boots into a Wayland compositor with our Rust shell and
  top panel.
- Full Linux driver tree (AMD, NVIDIA, Intel, nouveau, virtio, Hyper-V,
  VMware) is included.
- NetworkManager brings up wired + Wi-Fi.
- novai-pkg installs from the Arch repos via pacman.
- The desktop store (novai-launcher --store) shows a curated catalog
  with one-click install.

### What does NOT work yet

- The compositor renders a solid colour instead of client surfaces
  (Smithay GLES2 backend pending — see ROADMAP v0.2).
- The installer is not shipped yet — `novai-installer` is on the v0.5
  roadmap. For now, use `dd` to write the ISO and run it live.
- ARM64 builds are not produced.
- ISOs are not yet GPG-signed.

### Known issues

- On first boot in VirtualBox, you may need to disable 3D acceleration
  in the VM settings until the GLES2 backend lands.
- The lock screen greeter always logs in without password check on the
  live ISO — that is intentional for v0.1. Real PAM integration ships
  in v0.2.
- `novai-services` and `systemd` are both installed; the live ISO runs
  `systemd` as PID 1. To boot with `novai-services` instead, append
  `novai.init=/usr/bin/novai-services` to the kernel cmdline at the
  boot menu.

### Artifacts

- `novaios-<date>-x86_64.iso` — bootable ISO, ~1.8 GB.
- `novaios-<date>-x86_64.iso.sha256` — SHA-256 checksum.

### Verifying the download

```bash
sha256sum -c novaios-*.sha256
```

### Burning to USB

```bash
sudo dd if=novaios-*.iso of=/dev/sdX bs=4M conv=fsync status=progress
sync
```
