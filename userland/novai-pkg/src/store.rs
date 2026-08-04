//! The "store" — opens the novai-launcher desktop app and lets the user
//! one-click install. On a headless system it prints the catalog.

use anyhow::Result;
use std::process::Command;
use tracing::info;

pub async fn open_store() -> Result<()> {
    if let Ok(exe) = which::which("novai-launcher") {
        info!("launching store: {}", exe.display());
        Command::new(exe).arg("--store").spawn()?;
        return Ok(());
    }
    // Headless fallback: print a curated catalog.
    println!("NovaiOS Software Store (headless mode)\n");
    let catalog = [
        ("firefox",      "Mozilla Firefox — web browser"),
        ("chromium",     "Chromium — open-source browser"),
        ("vscode",       "Visual Studio Code (OSS build)"),
        ("gimp",         "GNU Image Manipulation Program"),
        ("vlc",          "VLC — media player"),
        ("obs-studio",   "Open Broadcaster Software"),
        ("steam",        "Steam — game client"),
        ("libreoffice",  "LibreOffice — office suite"),
        ("blender",      "Blender — 3D creation suite"),
        ("audacity",     "Audacity — audio editor"),
        ("docker",       "Docker — container runtime"),
        ("rustup",       "Rust toolchain installer"),
        ("nu",           "Nushell — modern Rust shell"),
        ("helix",        "Helix — modal text editor"),
        ("yazi",         "Yazi — terminal file manager"),
        ("bat",          "bat — cat(1) with wings"),
        ("fd",           "fd — friendly find(1)"),
        ("ripgrep",      "ripgrep — recursively search directories"),
        ("eza",          "eza — modern ls(1) replacement"),
        ("zoxide",       "zoxide — smarter cd"),
    ];
    for (slug, desc) in catalog {
        println!("  {:<14} {}", slug, desc);
    }
    println!("\nInstall with:  novai-pkg install <slug>");
    Ok(())
}
