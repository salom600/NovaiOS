//! Unit file model + parser.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub unit:     UnitMeta,
    pub service:  Service,
    pub install:  Install,
    pub path:     PathBuf,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UnitMeta {
    pub name:  String,
    pub desc:  String,
    #[serde(default)]
    pub after: Vec<String>,
    #[serde(default)]
    pub wants: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Service {
    pub exec:    String,
    #[serde(default)]
    pub restart: RestartPolicy,
    #[serde(default = "default_user")]
    pub user:    String,
    #[serde(default)]
    pub env:     HashMap<String, String>,
    #[serde(default)]
    pub cwd:     Option<String>,
}

fn default_user() -> String { "root".to_string() }

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RestartPolicy {
    #[default]
    Never,
    Always,
    OnFailure,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Install {
    #[serde(default)]
    pub wanted_by: Vec<String>,
}

impl Unit {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("read unit {}", path.display()))?;
        let mut u: Unit = toml::from_str(&raw)
            .with_context(|| format!("parse unit {}", path.display()))?;
        u.path = path.to_path_buf();
        if u.unit.name.is_empty() {
            u.unit.name = path.file_stem().unwrap().to_string_lossy().to_string();
        }
        Ok(u)
    }
}
