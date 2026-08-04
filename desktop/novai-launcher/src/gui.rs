//! GUI mode: iced launcher + store.

use crate::catalog::{catalog, AppEntry};
use anyhow::Result;
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{alignment, theme, Application, Command, Element, Length, Settings, Theme};
use std::process::Command as StdCommand;

pub fn run(store_mode: bool) -> Result<()> {
    NovaiLauncher::run(Settings {
        flags: store_mode,
        ..Default::default()
    })
    .map_err(|e| anyhow::anyhow!("iced: {e}"))
}

struct NovaiLauncher {
    store_mode: bool,
    query: String,
    catalog: Vec<AppEntry>,
    installing: Option<String>,
    log: String,
}

#[derive(Debug, Clone)]
enum Msg {
    Search(String),
    Launch(String),
    Install(String),
    InstallDone(String, bool),
}

impl Application for NovaiLauncher {
    type Message = Msg;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = bool; // store_mode

    fn new(store_mode: bool) -> (Self, Command<Msg>) {
        let s = Self {
            store_mode,
            query: String::new(),
            catalog: catalog(),
            installing: None,
            log: String::new(),
        };
        (s, Command::none())
    }
    fn title(&self, _id: iced::window::Id) -> String {
        if self.store_mode {
            "NovaiOS Store".into()
        } else {
            "NovaiOS Launcher".into()
        }
    }
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Search(s) => {
                self.query = s;
                Command::none()
            }
            Msg::Launch(slug) => {
                let _ = StdCommand::new(&slug).spawn();
                iced::window::close(iced::window::Id::MAIN)
            }
            Msg::Install(slug) => {
                if self.installing.is_some() {
                    return Command::none();
                }
                self.installing = Some(slug.clone());
                self.log = format!("Installing {}…", slug);
                Command::perform(install_async(slug.clone()), move |ok| {
                    Msg::InstallDone(slug, ok)
                })
            }
            Msg::InstallDone(slug, ok) => {
                self.installing = None;
                if ok {
                    self.log = format!("{} installed successfully.", slug);
                    if let Some(e) = self.catalog.iter_mut().find(|e| e.slug == slug) {
                        e.installed = true;
                    }
                } else {
                    self.log = format!("Failed to install {} — see logs.", slug);
                }
                Command::none()
            }
        }
    }
    fn view(&self, _id: iced::window::Id) -> Element<Msg> {
        let search = text_input("Search…", &self.query, Msg::Search)
            .padding(8)
            .width(Length::Fill);

        let q = self.query.to_lowercase();
        let apps: Vec<Element<_>> = self
            .catalog
            .iter()
            .filter(|a| {
                q.is_empty()
                    || a.name.to_lowercase().contains(&q)
                    || a.desc.to_lowercase().contains(&q)
                    || a.slug.contains(&q)
            })
            .map(|a| {
                let btn = if self.store_mode {
                    if a.installed {
                        button(text("Installed").size(14))
                            .padding(6)
                            .style(theme::Button::Secondary)
                    } else {
                        button(text("Install").size(14))
                            .on_press(Msg::Install(a.slug.clone()))
                            .padding(6)
                            .style(theme::Button::Primary)
                    }
                } else {
                    button(text("Open").size(14))
                        .on_press(Msg::Launch(a.slug.clone()))
                        .padding(6)
                        .style(theme::Button::Primary)
                };
                row![
                    text(format!("{}", a.icon)).size(22),
                    column![text(&a.name).size(15), text(&a.desc).size(12),].spacing(2),
                    btn,
                ]
                .spacing(12)
                .padding(8)
                .align_items(alignment::Alignment::Center)
                .into()
            })
            .collect();

        let body = column(
            std::iter::once(search)
                .chain(std::iter::once(text(&self.log).size(12)))
                .chain(apps)
                .collect::<Vec<_>>(),
        )
        .spacing(6);

        let scroll = scrollable(body).height(Length::Fill);

        let c = container(scroll)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .style(theme::Container::Box);
        c.into()
    }
    fn theme(&self, _id: iced::window::Id) -> Theme {
        Theme::Dark
    }
}

async fn install_async(slug: String) -> bool {
    StdCommand::new("novai-pkg")
        .args(["install", "--no-confirm", &slug])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
