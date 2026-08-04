//! Kernel cmdline parsing for novai-init.
//!
//! Recognised keys (all `novai.` prefixed):
//!   novai.live=1           — enable live ISO boot
//!   novai.root=<dev>:<fstype>:<opts>  — boot from this block device
//!   novai.init=<path>      — override /sbin/init
//!   novai.swap=<dev>       — early-enable this swap
//!   novai.debug=1          — verbose logging
//!   novai.squashfs=<path>  — explicit squashfs location
//!   novai.no_overlay=1     — skip the overlay (debug only)
//!   novai.install=1        — boot live then auto-launch the installer (Calamares)
//!
//! Also reads the standard `init=` and `root=` keys as a fallback.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Cmdline {
    pub live: bool,
    pub root: Option<String>,
    pub init: Option<String>,
    pub swap: Option<String>,
    pub squashfs: Option<String>,
    pub debug: bool,
    pub no_overlay: bool,
    pub install: bool,
    pub extra: std::collections::HashMap<String, String>,
}

impl Cmdline {
    /// Parse /proc/cmdline (or NOVAI_CMDLINE env for testing).
    pub fn parse() -> Option<Self> {
        let raw = if let Ok(s) = env::var("NOVAI_CMDLINE") {
            s
        } else {
            fs::read_to_string("/proc/cmdline").ok()?
        };
        Some(Self::parse_str(&raw))
    }

    pub fn parse_str(raw: &str) -> Self {
        let mut c = Cmdline::default();
        for tok in raw.split_whitespace() {
            let (k, v) = match tok.split_once('=') {
                Some((k, v)) => (k, Some(v)),
                None => (tok, None),
            };
            match k {
                "novai.live" => {
                    c.live = v
                        .map(|x| x == "1" || x.eq_ignore_ascii_case("true"))
                        .unwrap_or(true)
                }
                "novai.root" => c.root = v.map(str::to_owned),
                "novai.init" => c.init = v.map(str::to_owned),
                "novai.swap" => c.swap = v.map(str::to_owned),
                "novai.squashfs" => c.squashfs = v.map(str::to_owned),
                "novai.debug" => c.debug = v.map(|x| x == "1").unwrap_or(true),
                "novai.no_overlay" => c.no_overlay = v.map(|x| x == "1").unwrap_or(true),
                "novai.install" => c.install = v.map(|x| x == "1").unwrap_or(true),
                "init" => {
                    if c.init.is_none() {
                        c.init = v.map(str::to_owned);
                    }
                }
                "root" => {
                    if c.root.is_none() {
                        c.root = v.map(str::to_owned);
                    }
                }
                _ => {
                    if let Some(v) = v {
                        c.extra.insert(k.to_string(), v.to_string());
                    } else {
                        c.extra.insert(k.to_string(), "1".to_string());
                    }
                }
            }
        }
        if c.debug {
            std::env::set_var("RUST_LOG", "novai_init=debug");
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_live() {
        let c = Cmdline::parse_str("BOOT_IMAGE=/vmlinuz novai.live=1 novai.init=/sbin/init");
        assert!(c.live);
        assert_eq!(c.init.as_deref(), Some("/sbin/init"));
    }
    #[test]
    fn parses_real_root() {
        let c = Cmdline::parse_str("novai.root=/dev/sda2:ext4:rw");
        assert!(!c.live);
        assert_eq!(c.root.as_deref(), Some("/dev/sda2:ext4:rw"));
    }
    #[test]
    fn parses_install_flag() {
        let c = Cmdline::parse_str("novai.live=1 novai.install=1");
        assert!(c.live);
        assert!(c.install);
    }
}
