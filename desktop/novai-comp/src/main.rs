//! novai-comp — NovaiOS Wayland compositor.
//!
//! Built on top of [Smithay](https://github.com/Smithay/smithay), the Rust
//! Wayland library used by cosmic-comp,jay, and others.
//!
//! This is a *real* minimal but functional compositor:
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

use anyhow::{anyhow, Result};
use smithay::reexports::calloop::EventLoop;
use tracing::{info, warn};

mod backend;
mod shell;
mod render;
mod config;

fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("novai_comp=info,smithay=warn".parse().unwrap())
        .try_init();

    info!("novai-comp starting");

    let cfg = config::Config::load()
        .unwrap_or_else(|e| { warn!("config load failed ({}) — using defaults", e); config::Config::default() });
    info!("config: {}x{} @ {}Hz background={:?}",
          cfg.output.width, cfg.output.height,
          cfg.output.refresh, cfg.background.color);

    let mut event_loop: EventLoop<'static, backend::State> =
        EventLoop::try_new().map_err(|e| anyhow!("event loop: {e}"))?;
    let mut state = backend::State::new(&mut event_loop, cfg)?;

    // Main loop: dispatch libinput, wayland clients, and timer sources.
    let mut last_frame = std::time::Instant::now();
    loop {
        let wake = event_loop.dispatch(Some(std::time::Duration::from_millis(16)),
                                       &mut state)?;
        // Repaint any output that needs it.
        state.render_if_needed();
        if state.should_quit() {
            info!("quit requested");
            break;
        }
        last_frame = wake;
    }
    Ok(())
}
