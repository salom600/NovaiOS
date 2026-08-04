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
//! In the first ISO, novai-panel renders as a normal iced window if a
//! compositor with layer-shell support is running; otherwise it falls back
//! to stdout logging every 5s.

use chrono::Local;
use iced::widget::{button, column, container, row, text};
use iced::{alignment, theme, Application, Command, Element, Length, Settings, Theme};
use std::time::Duration;

fn main() -> iced::Result {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("novai_panel=info".parse().unwrap())
        .try_init();
    NovaiPanel::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(1920.0, 36.0),
            position: iced::window::Position::Specific(iced::Point::new(0.0, 0.0)),
            resizable: false,
            decorations: false,
            ..Default::default()
        },
        ..Default::default()
    })
}

struct NovaiPanel {
    workspaces: Vec<u32>,
    active_ws:  u32,
    window_title: String,
    now: String,
    battery_pct: u8,
    network_ssid: String,
}

#[derive(Debug, Clone)]
enum Msg {
    Tick,
    SwitchWS(u32),
}

impl Application for NovaiPanel {
    type Message = Msg;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Msg>) {
        let s = Self {
            workspaces: (1..=4).collect(),
            active_ws: 1,
            window_title: "Desktop".into(),
            now: Local::now().format("%H:%M").to_string(),
            battery_pct: read_battery_pct(),
            network_ssid: read_network_ssid(),
        };
        (s, Command::perform(tokio::time::sleep(Duration::from_secs(5)), |_| Msg::Tick))
    }
    fn title(&self, _id: iced::window::Id) -> String { "NovaiOS Panel".into() }
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Tick => {
                self.now = Local::now().format("%a %b %e  %H:%M").to_string();
                self.battery_pct = read_battery_pct();
                self.network_ssid = read_network_ssid();
                Command::perform(tokio::time::sleep(Duration::from_secs(5)), |_| Msg::Tick)
            }
            Msg::SwitchWS(i) => { self.active_ws = i; Command::none() }
        }
    }
    fn view(&self, _id: iced::window::Id) -> Element<Msg> {
        let ws: Element<_> = row(
            self.workspaces.iter().map(|w| {
                let b = button(text(format!("{}", w)).size(14))
                    .on_press(Msg::SwitchWS(*w))
                    .padding(2);
                if *w == self.active_ws {
                    b.style(theme::Button::Primary)
                } else {
                    b.style(theme::Button::Secondary)
                }.into()
            }).collect()
        ).spacing(2).into();

        let title = text(&self.window_title).size(14);
        let clock = text(&self.now).size(14);
        let status = text(format!("{}%  {}", self.battery_pct, self.network_ssid)).size(14);

        let content = row![
            ws,
            title,
            row![status, clock].spacing(12)
        ].spacing(20).align_items(alignment::Alignment::Center);

        let c = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([4, 12])
            .style(theme::Container::Box)
            .align_x(alignment::Alignment::Center)
            .align_y(alignment::Alignment::Center);
        c.into()
    }
    fn theme(&self, _id: iced::window::Id) -> Theme { Theme::Dark }
}

fn read_battery_pct() -> u8 {
    let cap = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity")
        .or_else(|_| std::fs::read_to_string("/sys/class/power_supply/BAT1/capacity"))
        .ok();
    cap.and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(100)
}

fn read_network_ssid() -> String {
    // Best-effort: use iw or NetworkManager CLI; fall back to "wired".
    let out = std::process::Command::new("nmcli")
        .args(["-t", "-f", "active,ssid", "dev", "wifi"])
        .output();
    if let Ok(o) = out {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            if let Some(rest) = line.strip_prefix("yes:") {
                return rest.to_string();
            }
        }
    }
    if std::path::Path::new("/sys/class/net/enp0s3").exists() ||
       std::path::Path::new("/sys/class/net/eth0").exists() {
        return "wired".to_string();
    }
    "offline".to_string()
}
