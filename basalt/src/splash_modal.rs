use std::marker::PhantomData;

use basalt_core::obsidian::Vault;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::Text,
    widgets::{Block, BorderType, Clear, StatefulWidget, Widget},
};

use crate::{
    app::Message as AppMessage,
    config::{symbol::Preset, Theme},
    vault_selector::{VaultSelector, VaultSelectorState},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Up,
    Down,
    Open,
}

pub fn update<'a>(message: &Message, state: &mut SplashModalState<'a>) -> Option<AppMessage<'a>> {
    match message {
        Message::Up => state.previous(),
        Message::Down => state.next(),
        Message::Open => {
            state.select();
            if let Some(vault) = state.selected_item() {
                state.hide();
                return Some(AppMessage::OpenVault(vault));
            }
        }
    };

    None
}

const TITLE: &str = "⋅𝕭𝖆𝖘𝖆𝖑𝖙⋅";
const TITLE_ASCII: &str = "Basalt";

pub const LOGO: [&str; 25] = [
    "           ▒███▓░          ",
    "          ▒█████▒░         ",
    "        ▒███▒██▓▒▒░        ",
    "      ▒████░██▓▒░▒▒░       ",
    "     ▒███▒▒██▒▒░ ░▒▒░      ",
    "   ▒████▓▓██▒░▒░  ░▒▒▒░    ",
    " ▒█████▓▓▓██ ░▒░  ░░▒▒▒░   ",
    "░████▓▓▒░░██ ░░ ░░░░░░▒▒░  ",
    "▒██▓▓▒░░░▒██░░▒░░░    ░▒░  ",
    "░███▓░░░░██▓░░▒▒▒▒░   ░▒▒  ",
    " ▒███░░░░██░░░░▒▒▒▒▒░░░▒▒  ",
    " ▒▒██▒░░░██░░░░░░░▒▒▒░ ░▒  ",
    " ▓▒░██░░▒█▓░░ ░░▒▒▒▒░ ░░▒  ",
    " █▒▒██▒░▓█░░ ░▒▒▒▒▒▒░ ░░▒░ ",
    "▒█▒▓▒██░██░▒▒▒▒▒░░░░ ░░░▒▒░",
    "▓█▒▓▒▓██▓█░░░░░░░░░  ░ ░░▒▒",
    "██▓▓▒▒▓█▓▓ ░░░░░░░░░░░░░░▒▒",
    "▒█▓▒░░ ▒▒▒░░░░ ░▒░░ ░░░▒▒▒░",
    "░▒▒▒░░░ ░░░░░░░░░░░░░░░▒▒░ ",
    " ░░▒▒░ ░ ░░░░░░░░░░░░▒▒░   ",
    "   ░▒▒▒░ ░ ░░░░░░░░▒▒░░    ",
    "     ░▒▒░░  ░░░░░░▒▒░      ",
    "       ░▒▒░░░░░▒▒▒▒░       ",
    "        ░░▒▒▒▒▒▒▒░         ",
    "          ░░▒▒░            ",
];

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SplashModalState<'a> {
    pub(crate) vault_selector_state: VaultSelectorState<'a>,
    pub(crate) version: &'a str,
    pub(crate) visible: bool,
}

impl<'a> SplashModalState<'a> {
    pub fn new(version: &'a str, items: Vec<&'a Vault>, visible: bool) -> Self {
        let vault_selector_state = VaultSelectorState::new(items);

        SplashModalState {
            version,
            vault_selector_state,
            visible,
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn select(&mut self) {
        self.vault_selector_state.select();
    }

    pub fn items(self) -> Vec<&'a Vault> {
        self.vault_selector_state.items
    }

    pub fn get_item(self, index: usize) -> Option<&'a Vault> {
        self.vault_selector_state.items.get(index).cloned()
    }

    pub fn selected_item(&self) -> Option<&'a Vault> {
        self.vault_selector_state
            .selected()
            .and_then(|index| self.vault_selector_state.items.get(index).cloned())
    }

    pub fn selected(&self) -> Option<usize> {
        self.vault_selector_state.selected()
    }

    pub fn next(&mut self) {
        self.vault_selector_state.next();
    }

    pub fn previous(&mut self) {
        self.vault_selector_state.previous();
    }
}

pub struct SplashModal<'a> {
    _lifetime: PhantomData<&'a ()>,
    pub border_type: BorderType,
    pub vault_active: String,
    pub theme: Theme,
    pub preset: Preset,
}

impl<'a> SplashModal<'a> {
    pub fn new(
        border_type: BorderType,
        vault_active: String,
        theme: Theme,
        preset: Preset,
    ) -> Self {
        Self {
            _lifetime: PhantomData,
            border_type,
            vault_active,
            theme,
            preset,
        }
    }
}

impl<'a> StatefulWidget for SplashModal<'a> {
    type State = SplashModalState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Clear.render(area, buf);
        Block::new()
            .style(Style::new().bg(self.theme.background))
            .render(area, buf);

        let [_, center, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(79),
            Constraint::Fill(1),
        ])
        .areas(area);

        let [_, top, bottom, _, help] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(28),
            Constraint::Min(6),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .flex(Flex::Center)
        .margin(1)
        .areas(center);

        let [logo, title] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(top);

        let [_, title, version] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .flex(Flex::SpaceBetween)
        .margin(1)
        .areas(title);

        let [bottom] = Layout::horizontal([Constraint::Length(60)])
            .flex(Flex::Center)
            .areas(bottom);

        let muted = self.theme.muted;
        Text::from_iter(LOGO).fg(muted).centered().render(logo, buf);

        let title_text = if self.preset == Preset::Ascii {
            TITLE_ASCII
        } else {
            TITLE
        };
        Text::from(title_text)
            .fg(muted)
            .centered()
            .render(title, buf);

        Text::from(state.version)
            .fg(muted)
            .italic()
            .centered()
            .render(version, buf);

        Text::from("Press (?) for help")
            .italic()
            .fg(muted)
            .centered()
            .render(help, buf);

        VaultSelector::new(self.border_type, self.vault_active, self.theme).render(
            bottom,
            buf,
            &mut state.vault_selector_state,
        );
    }
}
