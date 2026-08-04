//! novai-settings — system settings GUI.
//!
//! Tabs:
//!   • Appearance   (theme, accent colour, wallpaper)
//!   • Display      (resolution, scale, refresh rate)
//!   • Sound        (output device, volume)
//!   • Network      (Wi-Fi list, proxy)
//!   • Power        (perf mode via /sys/kernel/novai/perf_mode)
//!   • Users        (add/remove/lock user)
//!   • About        (NovaiOS version, kernel, CPU, RAM)
//!
//! Headless mode (no `gui` feature): prints system info + the current config.

mod config;

#[cfg(feature = "gui")]
mod gui;

#[cfg(not(feature = "gui"))]
mod headless;

use anyhow::Result;

fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("novai_settings=info"))
        .try_init();

    #[cfg(feature = "gui")]
    return gui::run();

    #[cfg(not(feature = "gui"))]
    return headless::run();
}
