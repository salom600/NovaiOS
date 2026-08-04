//! novai-lock — modern login greeter + screen locker.
//!
//! Two modes:
//!   • Default (no `--daemon`): full-screen iced greeter.
//!   • `--daemon`: listens on /run/novai/lock.sock for lock/unlock commands.
//!
//! Headless mode (no `gui` feature): prints the user list and waits.

mod users;

#[cfg(feature = "gui")]
mod gui;

#[cfg(not(feature = "gui"))]
mod headless;

use anyhow::Result;

fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("novai_lock=info"))
        .try_init();

    let daemon = std::env::args().any(|a| a == "--daemon");

    #[cfg(feature = "gui")]
    {
        if daemon {
            // Run a simple unix-socket server that listens for "lock\n".
            return gui::run_daemon();
        }
        return gui::run();
    }

    #[cfg(not(feature = "gui"))]
    {
        return headless::run(daemon);
    }
}
