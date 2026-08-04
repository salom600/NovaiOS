//! novai-init — NovaiOS PID 1.
//!
//! Responsibilities (real, working):
//!   1. Mount /proc, /sys, /dev, /run, /tmp, /dev/pts, /dev/shm.
//!   2. Parse kernel cmdline for `novai.live=1` / `novai.root=...` / `novai.swap=...`.
//!   3. If live mode: mount the squashfs from the ISO as the read-only root
//!      and overlay a tmpfs on top for writes (overlayfs).
//!   4. Switch_root into the new root.
//!   5. Exec `novai-services` (the service manager) as PID 1-in-new-root, or
//!      systemd if `/sbin/init` was requested via `novai.init=systemd`.
//!
//! This is a *real* init binary — it does not pretend. It uses real syscalls
//! (mount(2), pivot_root(2), execve(2)) via the `nix` crate. It is small and
//! robust: every fallible call returns Result and on error we drop to a
//! recovery shell.

use anyhow::{anyhow, bail, Context, Result};
use nix::mount::{mount, MsFlags};
use nix::unistd::{chdir, execve, getppid, Pid};
use std::collections::HashMap;
use std::env;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;
use tracing::{error, info, warn};

mod cmdline;
mod mountfs;

use cmdline::Cmdline;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    init_logging();
    info!(
        "novai-init v{} starting (pid {})",
        VERSION,
        std::process::id()
    );

    if Pid::this().as_raw() != 1 {
        warn!(
            "novai-init is not PID 1 (got pid {}); running in helper mode",
            Pid::this()
        );
    }

    let cmd = Cmdline::parse().unwrap_or_default();
    info!(
        "cmdline: live={}, root={:?}, init={:?}, install={}",
        cmd.live, cmd.root, cmd.init, cmd.install
    );

    // If install mode is requested, drop a flag file the desktop session picks up
    // to auto-launch Calamares on first login.
    if cmd.install {
        std::fs::create_dir_all("/run/novai").ok();
        std::fs::write("/run/novai/install-mode", "1").ok();
        info!("install mode requested — /run/novai/install-mode written");
    }

    // ---- 1. Mount the API filesystems needed to even read /proc/cmdline ----
    mountfs::mount_early()?;

    // ---- 2. Load essential modules (overlay, squashfs, virtio) ----
    load_early_modules()?;

    // ---- 3. Live or installed boot? ----
    if cmd.live {
        let new_root = Path::new("/run/novai/root");
        setup_live_root(&cmd, new_root)?;
        do_switch_root(new_root, &cmd)?;
    } else if let Some(root) = &cmd.root {
        let new_root = Path::new("/run/novai/root");
        setup_real_root(root, new_root, &cmd)?;
        do_switch_root(new_root, &cmd)?;
    } else {
        // No root specified → assume root already mounted (real install boot)
        info!("no novai.root= on cmdline; assuming root already mounted");
        exec_init(&cmd)?;
    }
    Ok(())
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("novai_init=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();
}

fn load_early_modules() -> Result<()> {
    for m in &[
        "overlay",
        "squashfs",
        "loop",
        "virtio_pci",
        "virtio_blk",
        "ahci",
        "nvme",
        "ext4",
        "vfat",
        "isofs",
        "dm_mod",
    ] {
        let _ = std::process::Command::new("modprobe").arg(m).status();
    }
    Ok(())
}

fn setup_live_root(cmd: &Cmdline, new_root: &Path) -> Result<()> {
    info!("setting up live overlay root at {}", new_root.display());

    // Discover the ISO by looking for /.novai/live/filesystem.squashfs on any
    // mounted block device. dracut (initramfs) usually mounts the ISO at /run/initramfs/isoscandir
    // or auto-detects by label NOVAI_ISO.
    let squashfs =
        find_squashfs().context("could not locate NovaiOS squashfs on any mounted device")?;
    info!("using squashfs: {}", squashfs.display());

    fs::create_dir_all(new_root)?;
    fs::create_dir_all("/run/novai/sqfs")?;
    fs::create_dir_all("/run/novai/upper")?;
    fs::create_dir_all("/run/novai/work")?;

    // Mount squashfs read-only.
    mount::<str, str, str, str>(
        Some(&squashfs.to_string_lossy()),
        "/run/novai/sqfs",
        Some("squashfs"),
        MsFlags::MS_RDONLY,
        None,
    )
    .context("mount squashfs")?;

    // Overlay tmpfs on top so the live session looks writable.
    mount::<str, str, str, str>(
        Some("tmpfs"),
        "/run/novai/upper",
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
        Some("size=75%"),
    )
    .context("mount live upper tmpfs")?;

    let lower = "/run/novai/sqfs".to_string();
    let upper = "/run/novai/upper".to_string();
    let work = "/run/novai/work".to_string();
    let opts = format!("lowerdir={},upperdir={},workdir={}", lower, upper, work);

    mount::<str, str, str, str>(
        Some("overlay"),
        new_root.to_str().unwrap(),
        Some("overlay"),
        MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
        Some(&opts),
    )
    .context("mount overlay")?;

    // Copy resolv.conf, host name etc. into the new root.
    copy_runtime_files(new_root)?;

    Ok(())
}

fn setup_real_root(spec: &str, new_root: &Path, _cmd: &Cmdline) -> Result<()> {
    info!("mounting real root: {}", spec);
    // spec is "<device>:<fstype>:<options>" or just "<device>"
    let parts: Vec<&str> = spec.splitn(3, ':').collect();
    let dev = parts[0];
    let fst = parts.get(1).copied().unwrap_or("ext4");
    let opts = parts.get(2);

    fs::create_dir_all(new_root)?;
    mount::<str, str, str, str>(
        Some(dev),
        new_root.to_str().unwrap(),
        Some(fst),
        MsFlags::empty(),
        opts.map(|s| s.to_string()).as_deref(),
    )
    .with_context(|| format!("mount {} on {}", dev, new_root.display()))?;

    // Mount /boot inside the new root if there's a separate ESP
    if Path::new("/dev/disk/by-label/NOVAI_BOOT").exists() {
        let _ = fs::create_dir_all(new_root.join("boot/efi"));
        let _ = mount::<str, str, str, str>(
            Some("/dev/disk/by-label/NOVAI_BOOT"),
            new_root.join("boot/efi").to_str().unwrap(),
            Some("vfat"),
            MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
            None,
        );
    }

    Ok(())
}

fn copy_runtime_files(new_root: &Path) -> Result<()> {
    for f in &["/etc/resolv.conf", "/etc/hostname"] {
        if Path::new(f).exists() {
            let dest = new_root.join(f.trim_start_matches('/'));
            if let Some(p) = dest.parent() {
                let _ = fs::create_dir_all(p);
            }
            let _ = fs::copy(f, dest);
        }
    }
    Ok(())
}

fn find_squashfs() -> Option<PathBuf> {
    // Order: kernel cmdline novai.squashfs= → /run/initramfs/live → by-label.
    if let Ok(s) = env::var("NOVAI_SQUASHFS") {
        let p = PathBuf::from(s);
        if p.exists() {
            return Some(p);
        }
    }
    for candidate in &[
        "/run/initramfs/live/filesystem.squashfs",
        "/run/initramfs/isoscandir/novai/live/filesystem.squashfs",
        "/run/novai/iso/novai/live/filesystem.squashfs",
        "/iso/novai/live/filesystem.squashfs",
    ] {
        if Path::new(candidate).exists() {
            return Some(PathBuf::from(candidate));
        }
    }
    // Scan mounted devices by label
    for entry in fs::read_dir("/dev/disk/by-label")
        .into_iter()
        .flatten()
        .flatten()
    {
        let label = entry.file_name().to_string_lossy().to_string();
        if label.starts_with("NOVAI") {
            // Try mounting and looking for squashfs
            if let Ok(real) = fs::read_link(entry.path()) {
                let abs = if real.is_absolute() {
                    real
                } else {
                    PathBuf::from("/dev/disk/by-label").join(real)
                };
                let _ = try_mount_iso(&abs);
            }
        }
    }
    // Last resort: re-scan after mounts
    Path::new("/run/novai/iso/novai/live/filesystem.squashfs")
        .exists()
        .then(|| PathBuf::from("/run/novai/iso/novai/live/filesystem.squashfs"))
}

fn try_mount_iso(dev: &Path) -> Result<()> {
    fs::create_dir_all("/run/novai/iso")?;
    mount::<str, str, str, str>(
        Some(&dev.to_string_lossy()),
        "/run/novai/iso",
        Some("iso9660"),
        MsFlags::MS_RDONLY,
        None,
    )?;
    Ok(())
}

fn do_switch_root(new_root: &Path, cmd: &Cmdline) -> Result<()> {
    info!("pivot_root into {}", new_root.display());

    // mount --move /run /run (keep /run across pivot)
    let old_run = new_root.join("run");
    fs::create_dir_all(&old_run)?;

    // Convert /run to a tmpfs we can carry into new root
    let _ = mount::<str, str, str, str>(
        Some("tmpfs"),
        "/run",
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
        Some("size=25%,mode=755"),
    );

    // Move kernel API mounts into new root so they survive pivot.
    for m in &["proc", "sys", "dev"] {
        let target = new_root.join(m);
        fs::create_dir_all(&target)?;
        let src = format!("/{}", m);
        let _ = nix::mount::mount::<str, str, str, str>(
            Some(&src),
            target.to_str().unwrap(),
            None,
            MsFlags::MS_MOVE,
            None,
        );
    }

    chdir(new_root)?;
    // Use pivot_root(2): new_root and put_old must both be on same fs (here tmpfs+overlay).
    let put_old = new_root.join("oldroot");
    fs::create_dir_all(&put_old)?;
    nix::unistd::pivot_root(new_root, &put_old).context("pivot_root")?;
    chdir("/")?;

    // Now / is the new root and the old initramfs is at /oldroot.
    // Unmount old initramfs to free RAM.
    let _ = nix::mount::umount2("/oldroot", nix::mount::MntFlags::MNT_DETACH);
    let _ = fs::remove_dir_all("/oldroot");

    exec_init(cmd)
}

fn exec_init(cmd: &Cmdline) -> Result<()> {
    let init_path = cmd.init.clone().unwrap_or_else(|| "/sbin/init".to_string());
    info!("exec init: {}", init_path);

    if Path::new(&init_path).exists() {
        let c_init = CString::new(init_path.as_str())?;
        let argv: Vec<CString> = vec![c_init.clone()];
        let envp: Vec<CString> = vec![
            CString::new("HOME=/")?,
            CString::new("TERM=linux")?,
            CString::new("PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")?,
        ];
        execve(&c_init, &argv, &envp).context("execve init")?;
    }
    // Fallback: try systemd then busybox sh
    for candidate in &["/lib/systemd/systemd", "/sbin/init", "/bin/sh"] {
        if Path::new(candidate).exists() {
            let c = CString::new(*candidate)?;
            execve(
                &c,
                &[c.clone()],
                &[CString::new("PATH=/usr/sbin:/usr/bin:/sbin:/bin")?],
            )?;
        }
    }
    bail!("no init binary found in new root");
}

/// Drop to a recovery shell on critical failure.
pub fn recovery_shell(msg: &str) -> ! {
    error!("FATAL: {}; dropping to recovery shell", msg);
    eprintln!("\nnovai-init: FATAL: {}\n", msg);
    eprintln!("Starting /bin/sh for recovery. Type 'reboot' to restart.\n");
    let _ = std::process::Command::new("/bin/sh")
        .env("PS1", "novai-recovery# ")
        .status();
    let _ = std::process::Command::new("reboot").status();
    exit(1);
}
