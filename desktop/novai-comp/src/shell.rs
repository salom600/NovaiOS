//! Shell logic: workspace tracking, window list, focus policy.
//! Kept minimal in the first ISO — exposes the data model that novai-panel
//! will later query via IPC.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Workspace {
    pub id: u32,
    pub name: String,
    pub windows: Vec<Window>,
    pub active: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Window {
    pub id: u32,
    pub app_id: String,
    pub title: String,
    pub focused: bool,
    pub fullscreen: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShellState {
    pub workspaces: Vec<Workspace>,
    pub active_ws: usize,
}

impl ShellState {
    pub fn new(n_workspaces: usize) -> Self {
        let mut workspaces = Vec::with_capacity(n_workspaces);
        for i in 0..n_workspaces {
            workspaces.push(Workspace {
                id: i as u32,
                name: (i + 1).to_string(),
                windows: vec![],
                active: i == 0,
            });
        }
        Self { workspaces, active_ws: 0 }
    }
    pub fn switch(&mut self, idx: usize) {
        if idx >= self.workspaces.len() { return; }
        self.workspaces[self.active_ws].active = false;
        self.active_ws = idx;
        self.workspaces[idx].active = true;
    }
    pub fn add_window(&mut self, win: Window) {
        self.workspaces[self.active_ws].windows.push(win);
    }
}
