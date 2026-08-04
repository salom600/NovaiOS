//! Filesystem mounting helpers used by novai-init.

use anyhow::{Context, Result};
use nix::mount::{mount, MsFlags};
use nix::sys::stat::Mode;
use nix::unistd::mkdir;
use std::path::Path;

/// Mount /proc, /sys, /dev, /run, /tmp, /dev/pts, /dev/shm, /dev/mqueue.
/// Idempotent — if a mountpoint is already a mount, the call is ignored.
pub fn mount_early() -> Result<()> {
    ensure_dir("/proc")?;
    ensure_dir("/sys")?;
    ensure_dir("/dev")?;
    ensure_dir("/run")?;
    ensure_dir("/tmp")?;
    ensure_dir("/dev/pts")?;
    ensure_dir("/dev/shm")?;
    ensure_dir("/dev/mqueue")?;

    try_mount(Some("proc"),     "/proc", "proc",     MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV, None);
    try_mount(Some("sysfs"),    "/sys",  "sysfs",    MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV, None);
    try_mount(Some("devtmpfs"), "/dev",  "devtmpfs", MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC, Some("mode=0755,size=4M"));
    try_mount(Some("tmpfs"),    "/run",  "tmpfs",    MsFlags::MS_NOSUID | MsFlags::MS_NODEV,    Some("mode=0755,size=25%"));
    try_mount(Some("tmpfs"),    "/tmp",  "tmpfs",    MsFlags::MS_NOSUID | MsFlags::MS_NODEV,    Some("mode=1777,size=25%"));
    try_mount(Some("devpts"),   "/dev/pts",  "devpts",  MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC, Some("gid=5,mode=620,ptmxmode=0666"));
    try_mount(Some("tmpfs"),    "/dev/shm",  "tmpfs",   MsFlags::MS_NOSUID | MsFlags::MS_NODEV, Some("mode=1777,size=10%"));
    try_mount(Some("mqueue"),   "/dev/mqueue","mqueue", MsFlags::MS_NOSUID | MsFlags::MS_NODEV, None);

    // /proc/sys/kernel/hostname
    let _ = std::fs::write("/proc/sys/kernel/hostname", "novai\n");
    Ok(())
}

pub fn ensure_dir(p: &str) -> Result<()> {
    if !Path::new(p).exists() {
        mkdir(p, Mode::from_bits_truncate(0o755))
            .with_context(|| format!("mkdir {}", p))?;
    }
    Ok(())
}

pub fn try_mount(
    source: Option<&str>,
    target: &str,
    fstype: &str,
    flags:  MsFlags,
    opts:   Option<&str>,
) {
    if let Err(e) = mount::<str, str, str, str>(source, target, Some(fstype), flags, opts) {
        // EBUSY = already mounted; ignore.
        if e != nix::errno::Errno::EBUSY {
            eprintln!("novai-init: mount {} ({}) failed: {}", target, fstype, e);
        }
    }
}
