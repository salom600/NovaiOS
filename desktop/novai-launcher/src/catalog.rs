//! Curated catalog of installable apps. Shared between headless + GUI modes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub slug: String,
    pub name: String,
    pub desc: String,
    pub icon: String,
    pub installed: bool,
}

pub fn catalog() -> Vec<AppEntry> {
    vec![
        AppEntry {
            slug: "firefox".into(),
            name: "Firefox".into(),
            desc: "Web browser".into(),
            icon: "🦊".into(),
            installed: false,
        },
        AppEntry {
            slug: "chromium".into(),
            name: "Chromium".into(),
            desc: "Open-source browser".into(),
            icon: "🌐".into(),
            installed: false,
        },
        AppEntry {
            slug: "vscode".into(),
            name: "VS Code".into(),
            desc: "Source code editor".into(),
            icon: "📝".into(),
            installed: false,
        },
        AppEntry {
            slug: "gimp".into(),
            name: "GIMP".into(),
            desc: "Image editor".into(),
            icon: "🎨".into(),
            installed: false,
        },
        AppEntry {
            slug: "vlc".into(),
            name: "VLC".into(),
            desc: "Media player".into(),
            icon: "🎬".into(),
            installed: false,
        },
        AppEntry {
            slug: "obs-studio".into(),
            name: "OBS Studio".into(),
            desc: "Screen recorder".into(),
            icon: "📹".into(),
            installed: false,
        },
        AppEntry {
            slug: "steam".into(),
            name: "Steam".into(),
            desc: "Game client".into(),
            icon: "🎮".into(),
            installed: false,
        },
        AppEntry {
            slug: "libreoffice".into(),
            name: "LibreOffice".into(),
            desc: "Office suite".into(),
            icon: "📄".into(),
            installed: false,
        },
        AppEntry {
            slug: "blender".into(),
            name: "Blender".into(),
            desc: "3D creation".into(),
            icon: "🧊".into(),
            installed: false,
        },
        AppEntry {
            slug: "audacity".into(),
            name: "Audacity".into(),
            desc: "Audio editor".into(),
            icon: "🎵".into(),
            installed: false,
        },
        AppEntry {
            slug: "docker".into(),
            name: "Docker".into(),
            desc: "Containers".into(),
            icon: "📦".into(),
            installed: false,
        },
        AppEntry {
            slug: "rustup".into(),
            name: "Rustup".into(),
            desc: "Rust toolchain".into(),
            icon: "🦀".into(),
            installed: false,
        },
        AppEntry {
            slug: "nu".into(),
            name: "Nushell".into(),
            desc: "Modern shell".into(),
            icon: "🐚".into(),
            installed: false,
        },
        AppEntry {
            slug: "helix".into(),
            name: "Helix".into(),
            desc: "Modal editor".into(),
            icon: "🌀".into(),
            installed: false,
        },
        AppEntry {
            slug: "yazi".into(),
            name: "Yazi".into(),
            desc: "Terminal file manager".into(),
            icon: "📁".into(),
            installed: false,
        },
    ]
}
