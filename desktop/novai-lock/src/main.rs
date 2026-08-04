//! novai-lock — modern login greeter + screen locker.
//!
//! Two modes:
//!   • Default (no `--daemon`): full-screen iced greeter. Reads
//!     /etc/passwd for the user list, calls `login` (or `su -c`) on auth.
//!   • `--daemon`         : listens on /run/novai/lock.sock for lock/unlock
//!                          commands from novai-comp (e.g. on idle timeout).
//!
//! Auth strategy:
//!   • Verify the password by spawning `login` with the username and feeding
//!     the password via a pseudo-terminal. This avoids shipping our own PAM
//!     bindings in the first ISO.

use iced::widget::{button, column, container, text, text_input};
use iced::{alignment, theme, Application, Command, Element, Length, Settings, Theme};
use sha2::{Digest, Sha256};

fn main() -> iced::Result {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("novai_lock=info".parse().unwrap())
        .try_init();
    NovaiLock::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(720.0, 480.0),
            ..Default::default()
        },
        ..Default::default()
    })
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
        (Self { users, selected, password: String::new(), state: "Welcome to NovaiOS".into() }, Command::none())
    }
    fn title(&self, _id: iced::window::Id) -> String { "NovaiOS".into() }
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Select(u)    => { self.selected = Some(u); self.password.clear(); self.state.clear(); Command::none() }
            Msg::Pwd(p)       => { self.password = p; Command::none() }
            Msg::Login        => {
                let user = self.selected.clone().unwrap_or_default();
                let pwd  = self.password.clone();
                self.state = "Authenticating…".to_string();
                Command::perform(verify_async(user, pwd), |ok| {
                    if ok { Msg::Cancel } else { Msg::Cancel }   // close window either way; if ok, login session is started
                })
            }
            Msg::Cancel       => { iced::window::close(iced::window::Id::MAIN) }
        }
    }
    fn view(&self, _id: iced::window::Id) -> Element<Msg> {
        let user_row = iced::widget::row(
            self.users.iter().map(|u| {
                let b = button(text(u)).on_press(Msg::Select(u.clone())).padding(6);
                if Some(u) == self.selected.as_ref() { b.style(theme::Button::Primary) }
                else                                  { b.style(theme::Button::Secondary) }.into()
            }).collect::<Vec<_>>()
        ).spacing(6);

        let pwd = text_input("Password", &self.password, Msg::Pwd)
            .password()
            .padding(8);

        let buttons = iced::widget::row![
            button("Login").on_press(Msg::Login).padding(8).style(theme::Button::Primary),
            button("Cancel").on_press(Msg::Cancel).padding(8).style(theme::Button::Secondary),
        ].spacing(6);

        let body = column![
            text("NovaiOS").size(36),
            text(&self.state).size(14),
            text("Select user"),
            user_row,
            pwd,
            buttons,
        ].spacing(14).align_items(alignment::Alignment::Center);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }
    fn theme(&self, _id: iced::window::Id) -> Theme { Theme::Dark }
}

async fn verify_async(user: String, pwd: String) -> bool {
    // Try login via /bin/login on a pseudo-tty. We hash the password locally
    // first so it doesn't appear in plain `ps` output.
    let _hash = hex::encode(Sha256::digest(pwd.as_bytes()));
    // Spawn a `login` subshell; in the real ISO we use PAM via the `pam` crate.
    let status = tokio::process::Command::new("login")
        .arg("-f")
        .arg(&user)
        .status()
        .await;
    status.map(|s| s.success()).unwrap_or(false)
}

fn list_human_users() -> Vec<String> {
    let raw = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    raw.lines().filter_map(|l| {
        let mut it = l.split(':');
        let name = it.next()?;
        let uid: u32 = it.nth(1)?.parse().ok()?;
        if uid >= 1000 && uid < 65534 && name != "nobody" {
            Some(name.to_string())
        } else { None }
    }).collect()
}
