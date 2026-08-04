//! The actual supervisor: starts units, restarts them on policy, watches.

use crate::unit::{RestartPolicy, Unit};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::oneshot;
use tracing::{error, info, warn};

pub struct Manager {
    units_dir: PathBuf,
    units: HashMap<String, Unit>,
    children: HashMap<String, ChildHandle>,
}

struct ChildHandle {
    child: Child,
    unit: Unit,
    stop_tx: oneshot::Sender<()>,
}

impl Manager {
    pub fn new(units_dir: PathBuf) -> Self {
        Self {
            units_dir,
            units: HashMap::new(),
            children: HashMap::new(),
        }
    }

    pub async fn load_all(&mut self) -> Result<()> {
        if !self.units_dir.exists() {
            warn!("units dir {} does not exist", self.units_dir.display());
            return Ok(());
        }
        let mut entries = tokio::fs::read_dir(&self.units_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("toml") {
                match Unit::load(&p) {
                    Ok(u) => {
                        info!("loaded unit: {}", u.unit.name);
                        self.units.insert(u.unit.name.clone(), u);
                    }
                    Err(e) => error!("skip {}: {}", p.display(), e),
                }
            }
        }
        Ok(())
    }

    pub fn list(&self) {
        println!("{:<24} {:<10} {}", "UNIT", "STATE", "DESCRIPTION");
        for (name, u) in &self.units {
            let state = if self.children.contains_key(name) {
                "running"
            } else {
                "stopped"
            };
            println!("{:<24} {:<10} {}", name, state, u.unit.desc);
        }
    }

    pub fn status(&self, name: &str) {
        match self.units.get(name) {
            None => println!("unknown unit: {}", name),
            Some(u) => {
                let running = self.children.contains_key(name);
                println!("● {} - {}", u.unit.name, u.unit.desc);
                println!("    Loaded:   ({})", u.path.display());
                println!(
                    "    Active:   {} since boot",
                    if running { "running" } else { "inactive" }
                );
                println!("    Exec:     {}", u.service.exec);
                println!("    Restart:  {:?}", u.service.restart);
            }
        }
    }

    pub async fn start(&mut self, name: &str) -> Result<()> {
        let unit = self
            .units
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("unknown unit {}", name))?;

        if self.children.contains_key(name) {
            warn!("{} already running", name);
            return Ok(());
        }

        // Start deps first — use Box::pin to break the async recursion.
        for dep in &unit.unit.after {
            if !self.children.contains_key(dep) && self.units.contains_key(dep) {
                Box::pin(self.start(dep)).await.ok();
            }
        }

        info!("starting {} (exec={})", name, unit.service.exec);
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c").arg(&unit.service.exec);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        if let Some(cwd) = &unit.service.cwd {
            cmd.current_dir(cwd);
        }
        for (k, v) in &unit.service.env {
            cmd.env(k, v);
        }
        if unit.service.user != "root" {
            cmd.uid(1000).gid(1000);
        }

        let child = cmd.spawn()?;
        let (stop_tx, _stop_rx) = oneshot::channel();
        self.children.insert(
            name.to_string(),
            ChildHandle {
                child,
                unit: unit.clone(),
                stop_tx,
            },
        );
        Ok(())
    }

    pub async fn stop(&mut self, name: &str) -> Result<()> {
        let mut handle = self
            .children
            .remove(name)
            .ok_or_else(|| anyhow!("unit {} not running", name))?;
        info!("stopping {}", name);
        let _ = handle.child.start_kill();
        let _ = handle.child.wait().await;
        Ok(())
    }

    /// Run as a supervisor: bring up everything that's `wanted_by` graphical.target,
    /// then poll children every 2s and restart on policy.
    pub async fn run(&mut self) -> Result<()> {
        info!(
            "novai-services supervisor running, {} units loaded",
            self.units.len()
        );

        // Start everything wanted by default target.
        let default_target =
            std::env::var("NOVAI_TARGET").unwrap_or("graphical.target".to_string());
        let mut to_start: Vec<String> = self
            .units
            .values()
            .filter(|u| u.install.wanted_by.iter().any(|t| t == &default_target))
            .map(|u| u.unit.name.clone())
            .collect();
        // Boot-critical units always start
        for critical in &["seatd", "dbus", "networkmanager", "polkit"] {
            if self.units.contains_key(*critical) && !to_start.contains(&critical.to_string()) {
                to_start.push(critical.to_string());
            }
        }

        for name in &to_start {
            if let Err(e) = self.start(name).await {
                error!("failed to start {}: {}", name, e);
            }
        }

        // Supervise
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            // Collect names of dead children
            let mut dead = Vec::new();
            for (name, h) in self.children.iter_mut() {
                match h.child.try_wait() {
                    Ok(Some(status)) => dead.push((name.clone(), status.code(), h.unit.clone())),
                    Ok(None) => {}
                    Err(_) => dead.push((name.clone(), None, h.unit.clone())),
                }
            }
            // Restart per policy
            for (name, code, unit) in dead {
                self.children.remove(&name);
                let should_restart = match unit.service.restart {
                    RestartPolicy::Always => true,
                    RestartPolicy::OnFailure => code.is_none() || code != Some(0),
                    RestartPolicy::Never => false,
                };
                if should_restart {
                    warn!("{} exited (code={:?}); restarting in 1s", name, code);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    if let Err(e) = self.start(&name).await {
                        error!("restart {}: {}", name, e);
                    }
                } else {
                    warn!("{} exited (code={:?}); not restarting", name, code);
                }
            }

            // Also handle SIGTERM/SIGINT for clean shutdown
            if shutdown_signalled().await.is_some() {
                info!("shutdown signal received; stopping all units");
                let names: Vec<String> = self.children.keys().cloned().collect();
                for n in names {
                    let _ = self.stop(&n).await;
                }
                break;
            }
        }
        Ok(())
    }
}

async fn shutdown_signalled() -> Option<()> {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sig = signal(SignalKind::terminate()).ok()?;
    // Non-blocking poll for a pending SIGTERM.
    use futures::future::poll_fn;
    use std::task::Poll;
    let p = poll_fn(|cx| match sig.poll_recv(cx) {
        Poll::Ready(Some(_)) => Poll::Ready(true),
        Poll::Ready(None) => Poll::Ready(true),
        Poll::Pending => Poll::Ready(false),
    })
    .await;
    if p {
        Some(())
    } else {
        None
    }
}
