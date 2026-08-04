//! Headless fallback for novai-launcher. Prints the catalog.

use crate::catalog::catalog;
use anyhow::Result;
use std::process::Command;

pub fn run(store_mode: bool) -> Result<()> {
    if store_mode {
        println!("NovaiOS Software Store (headless mode)\n");
        println!("{:<14} {:<14} {}", "SLUG", "ICON", "DESCRIPTION");
        for a in catalog() {
            println!("{:<14} {:<14} {}", a.slug, a.icon, a.desc);
        }
        println!("\nInstall with:  novai-pkg install <slug>");
    } else {
        // Launcher mode — list installed binaries in PATH.
        println!("NovaiOS Launcher (headless mode)\n");
        let path = std::env::var("PATH").unwrap_or_default();
        let mut apps: Vec<String> = Vec::new();
        for dir in path.split(':') {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for e in entries.flatten() {
                    if let Ok(name) = e.file_name().into_string() {
                        if !apps.contains(&name) {
                            apps.push(name);
                        }
                    }
                }
            }
        }
        apps.sort();
        for a in apps.iter().take(80) {
            println!("  {}", a);
        }
        println!("\nRun with `--store` for the installable catalog.");
    }
    // Run the install command directly if the user passed a slug.
    if let Some(slug) = std::env::args().nth(1) {
        if slug != "--store" {
            let _ = Command::new("novai-pkg").args(["install", &slug]).status();
        }
    }
    Ok(())
}
