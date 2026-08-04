//! Headless fallback: prints a status line to stdout every 5s.
//! Used when the `gui` feature is disabled.

use anyhow::Result;
use std::thread;
use std::time::Duration;

pub fn run() -> Result<()> {
    eprintln!("[novai-panel] running in headless mode (no `gui` feature)");
    loop {
        let now = chrono_like_now();
        let battery = read_battery_pct();
        let net = read_network_ssid();
        println!("[{}] ws=1 | {}% | {}", now, battery, net);
        thread::sleep(Duration::from_secs(5));
    }
}

fn chrono_like_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    format!("{:02}:{:02}", h, m)
}

fn read_battery_pct() -> u8 {
    let cap = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity")
        .or_else(|_| std::fs::read_to_string("/sys/class/power_supply/BAT1/capacity"))
        .ok();
    cap.and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(100)
}

fn read_network_ssid() -> String {
    let out = std::process::Command::new("nmcli")
        .args(["-t", "-f", "active,ssid", "dev", "wifi"])
        .output();
    if let Ok(o) = out {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            if let Some(rest) = line.strip_prefix("yes:") {
                return rest.to_string();
            }
        }
    }
    if std::path::Path::new("/sys/class/net/enp0s3").exists()
        || std::path::Path::new("/sys/class/net/eth0").exists()
    {
        return "wired".to_string();
    }
    "offline".to_string()
}
