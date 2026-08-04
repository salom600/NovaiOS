//! novai-installer — launches the NovaiOS installer.
//!
//! Strategy:
//!   1. Try Calamares (universal distro installer with a Qt GUI).
//!   2. If Calamares is missing or fails, fall back to `archinstall`
//!      (interactive TUI installer from Arch).
//!   3. If both are missing, print instructions for manual install.
//!
//! Used:
//!   - From the desktop's "Install NovaiOS" icon.
//!   - Automatically on first login if the kernel cmdline had
//!     `novai.install=1` (the bootloader's "Installation Mode" entry sets it).

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process::Command;
use tracing::info;

#[derive(Parser, Debug)]
#[command(
    name = "novai-installer",
    version,
    about = "NovaiOS installer launcher"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// Run in text mode (skip GUI, use archinstall TUI directly)
    #[arg(long, short = 't')]
    text: bool,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Launch the installer (default)
    Launch,
    /// Check if the installer dependencies are available
    Check,
    /// Print the install mode flag (1 if novai.install=1 was on cmdline)
    Mode,
}

fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("novai_installer=info"))
        .try_init();
    let cli = Cli::parse();

    match cli.cmd.unwrap_or(Cmd::Launch) {
        Cmd::Launch => launch(cli.text),
        Cmd::Check => check(),
        Cmd::Mode => print_mode(),
    }
}

fn launch(text_mode: bool) -> Result<()> {
    info!("launching NovaiOS installer (text_mode={})", text_mode);

    // Prefer Calamares in GUI mode
    if !text_mode && which::which("calamares").is_ok() {
        info!("found calamares — launching GUI installer");
        let status = Command::new("sudo").args(["-E", "calamares"]).status()?;
        if status.success() {
            println!("✅ Calamares finished successfully. Reboot to use your new system.");
            return Ok(());
        }
        eprintln!(
            "⚠️  Calamares exited with status {:?} — falling back to archinstall",
            status.code()
        );
    }

    // Fall back to archinstall (TUI)
    if which::which("archinstall").is_ok() {
        info!("launching archinstall (TUI fallback)");
        let status = Command::new("sudo").args(["-E", "archinstall"]).status()?;
        if status.success() {
            println!("✅ archinstall finished. Reboot to use your new system.");
            return Ok(());
        }
        anyhow::bail!("archinstall exited with status {:?}", status.code());
    }

    // Last resort: print manual instructions
    eprintln!("\n❌ No installer found. Install one of:");
    eprintln!("   sudo pacman -S calamares        # GUI installer");
    eprintln!("   sudo pacman -S archinstall      # TUI installer");
    eprintln!("\nOr partition manually with `cfdisk` then `pacstrap`.");
    std::process::exit(1);
}

fn check() -> Result<()> {
    let calamares = which::which("calamares").is_ok();
    let archinstall = which::which("archinstall").is_ok();
    let cfdisk = which::which("cfdisk").is_ok();
    let pacstrap = which::which("pacstrap").is_ok();
    println!("Installer check:");
    println!(
        "  calamares   : {}",
        if calamares {
            "✅ found"
        } else {
            "❌ missing"
        }
    );
    println!(
        "  archinstall : {}",
        if archinstall {
            "✅ found"
        } else {
            "❌ missing"
        }
    );
    println!(
        "  cfdisk      : {}",
        if cfdisk { "✅ found" } else { "❌ missing" }
    );
    println!(
        "  pacstrap    : {}",
        if pacstrap { "✅ found" } else { "❌ missing" }
    );
    Ok(())
}

fn print_mode() -> Result<()> {
    let install_mode = std::fs::read_to_string("/run/novai/install-mode")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if install_mode == "1" {
        println!("install");
    } else {
        println!("live");
    }
    Ok(())
}
