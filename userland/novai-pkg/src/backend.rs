//! Backend: thin Rust wrapper around `pacman` (and optional AUR helper later).

use anyhow::{Context, Result};
use std::process::Command;
use tracing::{info, warn};
use which::which;

pub struct Backend;

impl Backend {
    pub fn new() -> Result<Self> {
        if which("pacman").is_err() {
            warn!("pacman not found in PATH — novai-pkg will run in degraded mode");
        }
        Ok(Self)
    }

    pub async fn update(&self) -> Result<()> {
        info!("refreshing package index");
        run_pacman(&["-Sy"])?;
        Ok(())
    }

    pub async fn upgrade(&self, no_confirm: bool) -> Result<()> {
        let mut args = vec!["-Su"];
        if no_confirm {
            args.push("--noconfirm");
        }
        run_pacman(&args)?;
        Ok(())
    }

    pub async fn install(&self, names: &[String], no_confirm: bool) -> Result<()> {
        let mut args: Vec<String> = vec!["-S".into()];
        if no_confirm {
            args.push("--noconfirm".into());
        }
        args.extend(names.iter().cloned());
        let strs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_pacman(&strs)?;
        Ok(())
    }

    pub async fn remove(&self, names: &[String], no_confirm: bool) -> Result<()> {
        let mut args: Vec<String> = vec!["-R".into()];
        if no_confirm {
            args.push("--noconfirm".into());
        }
        args.extend(names.iter().cloned());
        let strs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_pacman(&strs)?;
        Ok(())
    }

    pub async fn search(&self, query: &str) -> Result<()> {
        run_pacman(&["-Ss", query])?;
        Ok(())
    }

    pub async fn info(&self, name: &str) -> Result<()> {
        run_pacman(&["-Si", name])?;
        Ok(())
    }

    pub async fn list(&self) -> Result<()> {
        run_pacman(&["-Q"])?;
        Ok(())
    }
}

fn run_pacman(args: &[&str]) -> Result<()> {
    let exe = which("pacman").context("pacman not found")?;
    let status = Command::new(exe)
        .args(args)
        .status()
        .with_context(|| format!("exec pacman {:?}", args))?;
    if !status.success() {
        anyhow::bail!("pacman {:?} failed: {}", args, status.code().unwrap_or(-1));
    }
    Ok(())
}
