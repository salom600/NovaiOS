//! Headless settings: print current config + system info.

use crate::config::{read_cpu_model, read_first_line, read_total_mem, read_uptime, Config};
use anyhow::Result;

pub fn run() -> Result<()> {
    let cfg = Config::load().unwrap_or_default();
    println!("=== NovaiOS Settings (headless) ===\n");
    println!("Appearance:");
    println!("  accent:    {}", cfg.accent);
    println!("  wallpaper: {}", cfg.wallpaper);
    println!("  dark_mode: {}", cfg.dark_mode);
    println!("\nPower:");
    println!("  perf_mode: {}", cfg.perf_mode);
    println!("\nAbout:");
    println!("  OS:       NovaiOS 0.1");
    println!("  Kernel:   {}", read_first_line("/proc/version"));
    println!("  Hostname: {}", read_first_line("/etc/hostname"));
    println!("  CPU:      {}", read_cpu_model());
    println!("  RAM:      {}", read_total_mem());
    println!("  Uptime:   {}", read_uptime());
    Ok(())
}
