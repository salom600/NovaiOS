//! novai-pkg — NovaiOS package manager front-end.
//!
//! Architecture:
//!   • Backend is `pacman` (proven, fast, signed) for system packages.
//!   • novai-pkg adds a friendly Rust CLI: `novai-pkg install firefox`,
//!     `novai-pkg search editor`, `novai-pkg update`/`upgrade`,
//!     `novai-pkg store` (one-click GUI store driven by this same binary).
//!   • All state under /var/lib/novai/pkg/.
//!   • Configuration in /etc/novai/pkg.toml.
//!
//! One-click install from the desktop store calls:
//!     novai-pkg install --no-confirm --from-store <slug>

mod backend;
mod store;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "novai-pkg",
    version,
    about = "NovaiOS package manager (pacman + Rust frontend)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    /// Don't ask for confirmation (used by the store).
    #[arg(long, global = true)]
    no_confirm: bool,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Update the local package index.
    Update,
    /// Upgrade all installed packages.
    Upgrade,
    /// Install one or more packages.
    Install { names: Vec<String> },
    /// Remove packages.
    Remove { names: Vec<String> },
    /// Search the package index.
    Search { query: String },
    /// Show info about a package.
    Info { name: String },
    /// List installed packages.
    List,
    /// Open the one-click store UI (used by the desktop launcher).
    Store,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("novai_pkg=info"))
        .try_init();
    let cli = Cli::parse();
    let be = backend::Backend::new()?;

    match cli.cmd {
        Cmd::Update => be.update().await,
        Cmd::Upgrade => be.upgrade(cli.no_confirm).await,
        Cmd::Install { names } => be.install(&names, cli.no_confirm).await,
        Cmd::Remove { names } => be.remove(&names, cli.no_confirm).await,
        Cmd::Search { query } => be.search(&query).await,
        Cmd::Info { name } => be.info(&name).await,
        Cmd::List => be.list().await,
        Cmd::Store => store::open_store().await,
    }
}
