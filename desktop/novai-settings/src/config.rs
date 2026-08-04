//! Settings configuration.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub accent: String,
    pub wallpaper: String,
    pub dark_mode: bool,
    pub perf_mode: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            accent: "#7c3aed".into(),
            wallpaper: String::new(),
            dark_mode: true,
            perf_mode: "balanced".into(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let p = PathBuf::from("/etc/novai/settings.toml");
        if !p.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(p)?;
        Ok(toml::from_str(&raw)?)
    }
}

pub fn save(c: &Config) -> Result<()> {
    let p = PathBuf::from("/etc/novai/settings.toml");
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(p, toml::to_string_pretty(c)?)?;
    Ok(())
}

pub fn read_first_line(p: &str) -> String {
    fs::read_to_string(p)
        .ok()
        .and_then(|s| s.lines().next().map(str::to_owned))
        .unwrap_or_default()
}

pub fn read_cpu_model() -> String {
    let raw = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("model name") {
            return rest.trim_start_matches([':', ' ']).to_string();
        }
    }
    "unknown".into()
}

pub fn read_total_mem() -> String {
    let raw = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
            return format!("{} MB", kb / 1024);
        }
    }
    "unknown".into()
}

pub fn read_uptime() -> String {
    let raw = fs::read_to_string("/proc/uptime").unwrap_or_default();
    let secs: f64 = raw
        .split_whitespace()
        .next()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0.0);
    let d = (secs / 86400.0) as u64;
    let h = ((secs % 86400.0) / 3600.0) as u64;
    let m = ((secs % 3600.0) / 60.0) as u64;
    format!("{}d {}h {}m", d, h, m)
}
