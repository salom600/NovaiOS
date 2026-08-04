//! Headless mode: prints users and waits for SIGTERM (acts as a no-op daemon
//! in `--daemon` mode so the compositor can still talk to the lock socket).

use crate::users::list_human_users;
use anyhow::Result;

pub fn run(daemon: bool) -> Result<()> {
    if daemon {
        eprintln!("[novai-lock] running as daemon (headless) — listening on /run/novai/lock.sock");
        let _ = std::fs::create_dir_all("/run/novai");
        let _ = std::fs::write("/run/novai/lock.sock", "novai-lock ready\n");
        // Block forever until SIGTERM.
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }
    println!("NovaiOS login (headless mode)\n");
    println!("Available users:");
    for u in list_human_users() {
        println!("  - {}", u);
    }
    println!("\nUse `login <user>` to switch (not implemented in headless mode).");
    Ok(())
}
