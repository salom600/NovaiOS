//! novai-panel — top-of-screen Wayland layer surface.
//!
//! Provides:
//!   • Workspace switcher (1..N)
//!   • Active window title
//!   • System tray (mock for now)
//!   • Clock + date
//!   • Battery + volume + network icons (read from /sys/class/)
//!
//! UI framework: iced (with the wayland backend).
//!
//! When built without the `gui` feature (the default for the first ISO),
//! novai-panel runs in headless mode and prints a status line every 5s.
//! This lets the panel ship in the ISO without pulling in iced's GPU stack.

#[cfg(not(feature = "gui"))]
mod headless;

#[cfg(feature = "gui")]
mod gui;

fn main() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("novai_panel=info"))
        .try_init();

    #[cfg(feature = "gui")]
    return gui::run();

    #[cfg(not(feature = "gui"))]
    return headless::run();
}
