//! novai-launcher — full-screen launcher + one-click software store.
//!
//! Two modes (selected by flag or window title):
//!   • Launcher   — fuzzy-find installed apps + recent files, Enter to launch.
//!   • Store      — curated catalog with [Install] buttons that call
//!                  `novai-pkg install --no-confirm <slug>`.
//!
//! When built without the `gui` feature (default for the first ISO), the
//! launcher prints the catalog to stdout. This lets the binary ship in the
//! ISO without pulling in iced's GPU stack.

mod catalog;

#[cfg(feature = "gui")]
mod gui;

#[cfg(not(feature = "gui"))]
mod headless;

fn main() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("novai_launcher=info"))
        .try_init();

    let store_mode = std::env::args().any(|a| a == "--store");

    #[cfg(feature = "gui")]
    return gui::run(store_mode);

    #[cfg(not(feature = "gui"))]
    {
        return headless::run(store_mode);
    }
}
