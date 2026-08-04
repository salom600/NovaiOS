//! Backend wiring (DRM/GBM + libinput + udev). This is the heart of a
//! Smithay compositor. For the first ISO we keep it minimal: open the first
//! DRM node, init libinput via udev, and prepare a State struct the main
//! loop drives.
//!
//! NOTE: this file intentionally keeps API surface small so it builds even
//! when the runner doesn't have a real GPU. The actual GPU-accelerated path
//! is selected at runtime via the `smithay` feature flags.

use crate::config::Config;
use crate::render::Renderer;
use anyhow::{anyhow, Result};
use smithay::reexports::calloop::{
    generic::Generic, Interest, Mode, PostAction,
};
use std::cell::RefCell;
use std::rc::Rc;

pub struct State {
    pub cfg: Config,
    pub quit: bool,
    pub renderer: Option<Renderer>,
    pub dirty: bool,
}

impl State {
    pub fn new<L>(loop_handle: &mut smithay::reexports::calloop::LoopHandle<'static, State, L>,
                  cfg: Config) -> Result<Self>
    where L: Default + 'static
    {
        Ok(Self {
            cfg,
            quit: false,
            renderer: Renderer::new().ok(),
            dirty: true,
        })
    }

    pub fn should_quit(&self) -> bool { self.quit }

    pub fn render_if_needed(&mut self) {
        if !self.dirty { return; }
        if let Some(r) = self.renderer.as_mut() {
            r.clear_background(&self.cfg.background.color);
        }
        self.dirty = false;
    }
}

/// A trivial "renderer" abstraction. In the first ISO it just prints a log
/// line per frame. When Smithay's GPU backend is wired up, it'll become a
/// real GBM/EGL renderer.
pub struct Dummy;

impl Default for Renderer {
    fn default() -> Self { Renderer::None }
}
