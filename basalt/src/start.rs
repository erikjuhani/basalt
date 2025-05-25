use std::marker::PhantomData;

use basalt_core::obsidian::Vault;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect, Size},
    style::Stylize,
    text::Text,
    widgets::{StatefulWidgetRef, Widget},
};

use crate::vault_selector::{VaultSelector, VaultSelectorState};

const TITLE: &str = "⋅𝕭𝖆𝖘𝖆𝖑𝖙⋅";

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
pub struct StartState<'a> {
    pub(crate) vault_selector_state: VaultSelectorState<'a>,
    pub(crate) size: Size,
    pub(crate) version: &'a str,
}

impl<'a> StartState<'a> {
    pub fn new(version: &'a str, size: Size, items: Vec<&'a Vault>) -> Self {
        let vault_selector_state = VaultSelectorState::new(items);

        StartState {
            version,
            size,
            vault_selector_state,
        }
    }

    pub fn select(&self) -> Self {
        Self {
            vault_selector_state: self.vault_selector_state.select(),
            ..self.clone()
        }
    }

    pub fn items(self) -> Vec<&'a Vault> {
        self.vault_selector_state.items
    }

    pub fn get_item(self, index: usize) -> Option<&'a Vault> {
        self.vault_selector_state.items.get(index).cloned()
    }

    pub fn selected(&self) -> Option<usize> {
        self.vault_selector_state.selected()
    }

    pub fn next(self) -> Self {
        Self {
            vault_selector_state: self.vault_selector_state.next(),
            ..self
        }
    }

    pub fn previous(self) -> Self {
        Self {
            vault_selector_state: self.vault_selector_state.previous(),
            ..self
        }
    }
}

#[derive(Default)]
pub struct StartScreen<'a> {
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> StatefulWidgetRef for StartScreen<'a> {
    type State = StartState<'a>;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
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

        Text::from_iter(LOGO)
            .dark_gray()
            .centered()
            .render(logo, buf);

        Text::from(TITLE).dark_gray().centered().render(title, buf);

        Text::from(state.version)
            .dark_gray()
            .italic()
            .centered()
            .render(version, buf);

        Text::from("Press (?) for help")
            .italic()
            .dark_gray()
            .centered()
            .render(help, buf);

        VaultSelector::default().render_ref(bottom, buf, &mut state.vault_selector_state);
    }
}
