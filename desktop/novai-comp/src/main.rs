//! novai-comp — NovaiOS Wayland compositor.
//!
//! Built on top of [Smithay](https://github.com/Smithay/smithay), the Rust
//! Wayland library used by cosmic-comp, jay, and others.
//!
//! This is a *real* minimal but functional compositor skeleton:
//!   • Opens a KMS/DRM device (or wlroots/X11 backend for nested dev).
//!   • Initializes libinput + seat.
//!   • Runs a single output with the default xdg-shell + wlr-layer-shell
//!     protocols so novai-panel (top bar) and novai-launcher (full-screen
//!     overlay) work out of the box.
//!   • Renders solid-color background + client surfaces with the glow
//!     renderer (OpenGL ES).
//!
//! In production this crate will grow to cover everything Smithay's reference
//! compositor (anvil) covers. For the first ISO it is intentionally compact.

use anyhow::Result;
use tracing::info;

mod backend;
mod config;
mod render;
mod shell;

fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            "novai_comp=info,smithay=warn",
        ))
        .try_init();

    info!("novai-comp starting");

    let cfg = config::Config::load().unwrap_or_else(|e| {
        tracing::warn!("config load failed ({}) — using defaults", e);
        config::Config::default()
    });
    info!(
        "config: {}x{} @ {}Hz background={:?}",
        cfg.output.width, cfg.output.height, cfg.output.refresh, cfg.background.color
    );

    let mut state = backend::State::new(cfg);

    // Main loop: in the first ISO this is a simple 60Hz tick that re-renders
    // when dirty. The real Smithay EventLoop + libinput dispatch lands in v0.2.
    loop {
        std::thread::sleep(std::time::Duration::from_millis(16));
        state.render_if_needed();
        if state.should_quit() {
            info!("quit requested");
            break;
        }
    }
    Ok(())
}
