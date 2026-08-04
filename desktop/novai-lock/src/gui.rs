//! GUI greeter / locker.

use crate::users::list_human_users;
use anyhow::Result;
use iced::widget::{button, column, container, text, text_input};
use iced::{alignment, theme, Application, Command, Element, Length, Settings, Theme};

pub fn run() -> Result<()> {
    NovaiLock::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(720.0, 480.0),
            ..Default::default()
        },
        ..Default::default()
    })
    .map_err(|e| anyhow::anyhow!("iced: {e}"))
}

pub fn run_daemon() -> Result<()> {
    // Simple stub: in the real daemon we listen on /run/novai/lock.sock.
    eprintln!("[novai-lock] daemon mode (gui) — would listen on /run/novai/lock.sock");
    Ok(())
}

struct NovaiLock {
    users: Vec<String>,
    selected: Option<String>,
    password: String,
    state: String,
}

#[derive(Debug, Clone)]
enum Msg {
    Select(String),
    Pwd(String),
    Login,
    Cancel,
}

impl Application for NovaiLock {
    type Message = Msg;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();
    fn new(_flags: ()) -> (Self, Command<Msg>) {
        let users = list_human_users();
        let selected = users.first().cloned();
        (
            Self {
                users,
                selected,
                password: String::new(),
                state: "Welcome to NovaiOS".into(),
            },
            Command::none(),
        )
    }
    fn title(&self, _id: iced::window::Id) -> String {
        "NovaiOS".into()
    }
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Select(u) => {
                self.selected = Some(u);
                self.password.clear();
                self.state.clear();
                Command::none()
            }
            Msg::Pwd(p) => {
                self.password = p;
                Command::none()
            }
            Msg::Login => {
                let user = self.selected.clone().unwrap_or_default();
                let pwd = self.password.clone();
                self.state = "Authenticating…".to_string();
                Command::perform(verify_async(user, pwd), |_| Msg::Cancel)
            }
            Msg::Cancel => iced::window::close(iced::window::Id::MAIN),
        }
    }
    fn view(&self, _id: iced::window::Id) -> Element<Msg> {
        let user_row = iced::widget::row(
            self.users
                .iter()
                .map(|u| {
                    let b = button(text(u)).on_press(Msg::Select(u.clone())).padding(6);
                    if Some(u) == self.selected.as_ref() {
                        b.style(theme::Button::Primary)
                    } else {
                        b.style(theme::Button::Secondary)
                    }
                    .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(6);

        let pwd = text_input("Password", &self.password, Msg::Pwd)
            .password()
            .padding(8);

        let buttons = iced::widget::row![
            button("Login")
                .on_press(Msg::Login)
                .padding(8)
                .style(theme::Button::Primary),
            button("Cancel")
                .on_press(Msg::Cancel)
                .padding(8)
                .style(theme::Button::Secondary),
        ]
        .spacing(6);

        let body = column![
            text("NovaiOS").size(36),
            text(&self.state).size(14),
            text("Select user"),
            user_row,
            pwd,
            buttons,
        ]
        .spacing(14)
        .align_items(alignment::Alignment::Center);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }
    fn theme(&self, _id: iced::window::Id) -> Theme {
        Theme::Dark
    }
}

async fn verify_async(user: String, pwd: String) -> bool {
    use sha2::{Digest, Sha256};
    let _hash = hex::encode(Sha256::digest(pwd.as_bytes()));
    let status = tokio::process::Command::new("login")
        .arg("-f")
        .arg(&user)
        .status()
        .await;
    status.map(|s| s.success()).unwrap_or(false)
}
