//! GUI mode: iced top bar.

use anyhow::Result;
use iced::widget::{button, container, row, text};
use iced::{alignment, theme, Application, Command, Element, Length, Settings, Theme};
use std::time::Duration;

pub fn run() -> Result<()> {
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
    .map_err(|e| anyhow::anyhow!("iced: {e}"))
}

struct NovaiPanel {
    workspaces: Vec<u32>,
    active_ws: u32,
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
            now: now_str(),
            battery_pct: read_battery_pct(),
            network_ssid: read_network_ssid(),
        };
        (
            s,
            Command::perform(tokio::time::sleep(Duration::from_secs(5)), |_| Msg::Tick),
        )
    }
    fn title(&self, _id: iced::window::Id) -> String {
        "NovaiOS Panel".into()
    }
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Tick => {
                self.now = now_str();
                self.battery_pct = read_battery_pct();
                self.network_ssid = read_network_ssid();
                Command::perform(tokio::time::sleep(Duration::from_secs(5)), |_| Msg::Tick)
            }
            Msg::SwitchWS(i) => {
                self.active_ws = i;
                Command::none()
            }
        }
    }
    fn view(&self, _id: iced::window::Id) -> Element<Msg> {
        let ws: Element<_> = row(self
            .workspaces
            .iter()
            .map(|w| {
                let b = button(text(format!("{}", w)).size(14))
                    .on_press(Msg::SwitchWS(*w))
                    .padding(2);
                if *w == self.active_ws {
                    b.style(theme::Button::Primary)
                } else {
                    b.style(theme::Button::Secondary)
                }
                .into()
            })
            .collect())
        .spacing(2)
        .into();

        let title = text(&self.window_title).size(14);
        let clock = text(&self.now).size(14);
        let status = text(format!("{}%  {}", self.battery_pct, self.network_ssid)).size(14);

        let content = row![ws, title, row![status, clock].spacing(12)]
            .spacing(20)
            .align_items(alignment::Alignment::Center);

        let c = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([4, 12])
            .style(theme::Container::Box)
            .align_x(alignment::Alignment::Center)
            .align_y(alignment::Alignment::Center);
        c.into()
    }
    fn theme(&self, _id: iced::window::Id) -> Theme {
        Theme::Dark
    }
}

fn now_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    format!("{:02}:{:02}", h, m)
}

fn read_battery_pct() -> u8 {
    let cap = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity")
        .or_else(|_| std::fs::read_to_string("/sys/class/power_supply/BAT1/capacity"))
        .ok();
    cap.and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(100)
}

fn read_network_ssid() -> String {
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
    if std::path::Path::new("/sys/class/net/enp0s3").exists()
        || std::path::Path::new("/sys/class/net/eth0").exists()
    {
        return "wired".to_string();
    }
    "offline".to_string()
}
