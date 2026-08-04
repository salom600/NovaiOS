//! Backend wiring (DRM/GBM + libinput + udev). This is the heart of a
//! Smithay compositor. For the first ISO we keep it minimal: prepare a
//! State struct the main loop drives. The real GPU path is selected at
//! runtime via Smithay's feature flags.

use crate::config::Config;
use crate::render::Renderer;

pub struct State {
    pub cfg: Config,
    pub quit: bool,
    pub renderer: Option<Renderer>,
    pub dirty: bool,
}

impl State {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            quit: false,
            renderer: Renderer::new().ok(),
            dirty: true,
        }
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn render_if_needed(&mut self) {
        if !self.dirty {
            return;
        }
        if let Some(r) = self.renderer.as_mut() {
            r.clear_background(&self.cfg.background.color);
        }
        self.dirty = false;
    }
}
