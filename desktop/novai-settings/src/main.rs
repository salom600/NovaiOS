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

use iced::widget::{button, column, container, row, scrollable, text, text_input, toggler};
use iced::{alignment, theme, Application, Command, Element, Length, Settings, Theme};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

fn main() -> iced::Result {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("novai_settings=info".parse().unwrap())
        .try_init();
    NovaiSettings::run(Settings::default())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab { Appearance, Display, Sound, Network, Power, Users, About }

struct NovaiSettings {
    tab: Tab,
    accent: String,
    wallpaper: String,
    dark_mode: bool,
    perf_mode: String,
    hostname: String,
    about: String,
}

#[derive(Debug, Clone)]
enum Msg {
    Switch(Tab),
    SetAccent(String),
    SetWallpaper(String),
    ToggleDark(bool),
    SetPerf(String),
    SaveConfig,
}

impl Application for NovaiSettings {
    type Message = Msg;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Msg>) {
        let cfg = load_config().unwrap_or_default();
        let about = format!(
            "NovaiOS 0.1.0\n\
             Kernel: {}\n\
             Hostname: {}\n\
             CPU: {}\n\
             RAM: {}\n\
             Uptime: {}",
            read_first_line("/proc/version"),
            read_first_line("/etc/hostname"),
            read_cpu_model(),
            read_total_mem(),
            read_uptime(),
        );
        (Self {
            tab: Tab::Appearance,
            accent: cfg.accent,
            wallpaper: cfg.wallpaper,
            dark_mode: cfg.dark_mode,
            perf_mode: cfg.perf_mode,
            hostname: read_first_line("/etc/hostname"),
            about,
        }, Command::none())
    }
    fn title(&self, _id: iced::window::Id) -> String { "NovaiOS Settings".into() }
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Switch(t)      => { self.tab = t; Command::none() }
            Msg::SetAccent(s)   => { self.accent = s; Command::none() }
            Msg::SetWallpaper(s)=> { self.wallpaper = s; Command::none() }
            Msg::ToggleDark(b)  => { self.dark_mode = b; Command::none() }
            Msg::SetPerf(s)     => { self.perf_mode = s; Command::none() }
            Msg::SaveConfig     => {
                let cfg = Config {
                    accent: self.accent.clone(),
                    wallpaper: self.wallpaper.clone(),
                    dark_mode: self.dark_mode,
                    perf_mode: self.perf_mode.clone(),
                };
                let _ = save_config(&cfg);
                Command::none()
            }
        }
    }
    fn view(&self, _id: iced::window::Id) -> Element<Msg> {
        let nav = column(vec![
            nav_btn("Appearance", Tab::Appearance, self.tab),
            nav_btn("Display",    Tab::Display,    self.tab),
            nav_btn("Sound",      Tab::Sound,      self.tab),
            nav_btn("Network",    Tab::Network,    self.tab),
            nav_btn("Power",      Tab::Power,      self.tab),
            nav_btn("Users",      Tab::Users,      self.tab),
            nav_btn("About",      Tab::About,      self.tab),
        ]).spacing(2);

        let body = match self.tab {
            Tab::Appearance => column![
                text("Appearance").size(20),
                text("Accent colour"),
                text_input("Hex colour", &self.accent, Msg::SetAccent).padding(6),
                text("Wallpaper path"),
                text_input("/path/to/wallpaper.png", &self.wallpaper, Msg::SetWallpaper).padding(6),
                row![text("Dark mode"), toggler(self.dark_mode, Msg::ToggleDark)].spacing(12),
                button("Save").on_press(Msg::SaveConfig).padding(8).style(theme::Button::Primary),
            ].spacing(8).into(),
            Tab::Display => column![text("Display").size(20), text("Resolution, refresh rate, scale live here.")].into(),
            Tab::Sound   => column![text("Sound").size(20),   text("Output device + volume live here.")].into(),
            Tab::Network => column![text("Network").size(20), text("Wi-Fi list + proxy live here.")].into(),
            Tab::Power   => column![
                text("Power").size(20),
                text("Performance mode"),
                row![
                    perf_btn("balanced",   &self.perf_mode),
                    perf_btn("performance",&self.perf_mode),
                    perf_btn("powersave",  &self.perf_mode),
                ].spacing(6),
                button("Save").on_press(Msg::SaveConfig).padding(8).style(theme::Button::Primary),
            ].spacing(8).into(),
            Tab::Users => column![text("Users").size(20), text("Add/remove/lock user lives here.")].into(),
            Tab::About => column![text("About").size(20), text(&self.about).size(14)].spacing(8).into(),
        };

        let layout = row![
            container(nav).width(160).height(Length::Fill).padding(8).style(theme::Container::Box),
            container(scrollable(body)).width(Length::Fill).height(Length::Fill).padding(16),
        ].spacing(8);

        container(layout).width(Length::Fill).height(Length::Fill).padding(8).into()
    }
    fn theme(&self, _id: iced::window::Id) -> Theme { if self.dark_mode { Theme::Dark } else { Theme::Light } }
}

fn nav_btn(label: &str, t: Tab, current: Tab) -> Element<'static, Msg> {
    let b = button(text(label).size(14))
        .on_press(Msg::Switch(t))
        .padding(6)
        .width(Length::Fill);
    if t == current { b.style(theme::Button::Primary) } else { b.style(theme::Button::Secondary) }.into()
}

fn perf_btn(label: &str, current: &str) -> Element<'static, Msg> {
    let b = button(text(label).size(14))
        .on_press(Msg::SetPerf(label.into()))
        .padding(6);
    if label == current { b.style(theme::Button::Primary) } else { b.style(theme::Button::Secondary) }.into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Config {
    accent: String,
    wallpaper: String,
    dark_mode: bool,
    perf_mode: String,
}

fn load_config() -> anyhow::Result<Config> {
    let p = PathBuf::from("/etc/novai/settings.toml");
    if !p.exists() { return Ok(Config { accent: "#7c3aed".into(), wallpaper: String::new(), dark_mode: true, perf_mode: "balanced".into() }); }
    let raw = fs::read_to_string(p)?;
    Ok(toml::from_str(&raw)?)
}

fn save_config(c: &Config) -> anyhow::Result<()> {
    let p = PathBuf::from("/etc/novai/settings.toml");
    if let Some(parent) = p.parent() { fs::create_dir_all(parent)?; }
    fs::write(p, toml::to_string_pretty(c)?)?;
    Ok(())
}

fn read_first_line(p: &str) -> String {
    fs::read_to_string(p).ok()
        .and_then(|s| s.lines().next().map(str::to_owned))
        .unwrap_or_default()
}

fn read_cpu_model() -> String {
    let raw = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("model name") {
            return rest.trim_start_matches([':', ' ']).to_string();
        }
    }
    "unknown".into()
}

fn read_total_mem() -> String {
    let raw = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
            return format!("{} MB", kb / 1024);
        }
    }
    "unknown".into()
}

fn read_uptime() -> String {
    let raw = fs::read_to_string("/proc/uptime").unwrap_or_default();
    let secs: f64 = raw.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0.0);
    let d = (secs / 86400.0) as u64; let h = ((secs % 86400.0) / 3600.0) as u64; let m = ((secs % 3600.0) / 60.0) as u64;
    format!("{}d {}h {}m", d, h, m)
}
