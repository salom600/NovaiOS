//! novai-comp configuration.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use toml;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub output: OutputConfig,
    pub background: BackgroundConfig,
    pub input: InputConfig,
    pub workspaces: usize,
    pub autostart: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub scale: f32,
    pub transform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundConfig {
    pub color: [u8; 3], // RGB
    pub image: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub tap_to_click: bool,
    pub natural_scroll: bool,
    pub accel_profile: String, // "adaptive" | "flat"
    pub repeat_rate: u32,
    pub repeat_delay: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output: OutputConfig {
                width: 1920,
                height: 1080,
                refresh: 60,
                scale: 1.0,
                transform: "normal".into(),
            },
            background: BackgroundConfig {
                color: [16, 18, 28],
                image: None,
            },
            input: InputConfig {
                tap_to_click: true,
                natural_scroll: true,
                accel_profile: "adaptive".into(),
                repeat_rate: 25,
                repeat_delay: 250,
            },
            workspaces: 4,
            autostart: vec![
                "/usr/bin/novai-panel".into(),
                "/usr/bin/novai-lock --daemon".into(),
                "/usr/bin/waybar".into(),
            ],
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let paths = [
            PathBuf::from("/etc/novai/comp.toml"),
            dirs::config_dir()
                .map(|d| d.join("novai/comp.toml"))
                .unwrap_or_default(),
        ];
        for p in paths {
            if p.exists() {
                let raw = std::fs::read_to_string(&p)?;
                return Ok(toml::from_str(&raw)?);
            }
        }
        Ok(Self::default())
    }
}

#[allow(unused)]
mod dirs_shim {
    pub fn config_dir() -> Option<std::path::PathBuf> {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| std::path::PathBuf::from(h).join(".config"))
            })
    }
}
use dirs_shim as dirs;
