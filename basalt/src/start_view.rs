use basalt_widgets::vault::VaultSelector;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::Stylize,
    text::Text,
    widgets::{StatefulWidgetRef, Widget},
};

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
pub struct StartView<'a> {
    pub start_state: StartViewState<'a>,
}

impl<'a> StatefulWidgetRef for StartView<'a> {
    type State = StartViewState<'a>;

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

        Text::from_iter(LOGO).white().centered().render(logo, buf);

        Text::from(TITLE).white().centered().render(title, buf);

        Text::from(state.version)
            .white()
            .italic()
            .centered()
            .render(version, buf);

        Text::from("Press (?) for help")
            .white()
            .italic()
            .centered()
            .render(help, buf);

        VaultSelector::default().render_ref(bottom, buf, &mut state.vault_selector_state);
    }
}

use basalt_core::obsidian::Vault;
use basalt_widgets::vault::VaultSelectorState;
use ratatui::layout::Size;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct StartViewState<'a> {
    pub(crate) vault_selector_state: VaultSelectorState<'a>,
    pub(crate) size: Size,
    pub(crate) version: &'a str,
}

impl<'a> StartViewState<'a> {
    pub fn new(version: &'a str, size: Size, items: Vec<&'a Vault>) -> Self {
        let vault_selector_state = VaultSelectorState::new(items);

        StartViewState {
            version,
            size,
            vault_selector_state,
        }
    }

    pub fn select(&mut self) {
        self.vault_selector_state.select();
    }

    pub fn items(self) -> Vec<&'a Vault> {
        self.vault_selector_state.items
    }

    pub fn get_item(&self, index: usize) -> Option<&'a Vault> {
        self.vault_selector_state.items.get(index).cloned()
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
