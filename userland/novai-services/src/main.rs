//! novai-services — service manager for NovaiOS.
//!
//! A small supervised process manager that reads unit files from
//! /etc/novai/services/*.toml and brings the system up to a usable state.
//!
//! Unit file format (TOML):
//! ```toml
//! [unit]
//! name    = "novai-comp"
//! desc    = "Wayland compositor"
//! after   = ["network.target", "seatd.service"]
//!
//! [service]
//! exec    = "/usr/bin/novai-comp"
//! restart = "always"        # always | on-failure | never
//! user    = "novai"
//! env     = { WAYLAND_DISPLAY = "wayland-0", XDG_RUNTIME_DIR = "/run/user/1000" }
//!
//! [install]
//! wanted_by = ["graphical.target"]
//! ```
//!
//! novai-services is intentionally *not* a drop-in systemd replacement — it
//! is small (a few hundred lines) and exists to be the supervision layer
//! when the user has opted out of systemd via `novai.init=/sbin/novai-services`.

mod manager;
mod unit;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "novai-services", version, about = "NovaiOS service manager")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// Unit directory
    #[arg(long, default_value = "/etc/novai/services")]
    dir: PathBuf,
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run as PID-1-style supervisor (default if no subcommand given).
    Run,
    /// List loaded units.
    List,
    /// Start a unit by name.
    Start   { name: String },
    /// Stop a unit by name.
    Stop    { name: String },
    /// Restart a unit by name.
    Restart { name: String },
    /// Show status of a unit.
    Status  { name: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            if cli.verbose { "debug" } else { "info" }.parse().unwrap()
        )
        .try_init();

    let mut mgr = manager::Manager::new(cli.dir.clone());
    mgr.load_all().await?;

    match cli.cmd.unwrap_or(Cmd::Run) {
        Cmd::Run       => mgr.run().await,
        Cmd::List      => { mgr.list(); Ok(()) }
        Cmd::Start{n}  => mgr.start(&n).await,
        Cmd::Stop{n}   => mgr.stop(&n).await,
        Cmd::Restart{n}=> { mgr.stop(&n).await?; mgr.start(&n).await }
        Cmd::Status{n} => { mgr.status(&n); Ok(()) }
    }
}
