use std::marker::PhantomData;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    widgets::{BorderType, Clear, StatefulWidget, Widget},
};

use crate::{
    app::Message as AppMessage,
    config::Theme,
    theme_selector::{ThemeSelector, ThemeSelectorState},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Toggle,
    Up,
    Down,
    Select,
    Close,
}

/// Drives the theme picker. Navigating previews the highlighted theme live;
/// `Select` keeps it, while `Close` (and toggling shut) reverts to whatever was
/// active when the picker opened, mirroring Helix's `:theme`.
pub fn update<'a>(
    message: &Message,
    state: &mut ThemeSelectorModalState,
    current: Theme,
) -> Option<AppMessage<'a>> {
    match message {
        Message::Toggle if !state.visible => {
            state.open(current);
            state.preview()
        }
        Message::Toggle | Message::Close => {
            state.hide();
            Some(AppMessage::PreviewTheme(state.original))
        }
        Message::Up => {
            state.theme_selector_state.previous();
            state.preview()
        }
        Message::Down => {
            state.theme_selector_state.next();
            state.preview()
        }
        Message::Select => {
            let name = state
                .theme_selector_state
                .selected_name()
                .map(str::to_string);
            state.hide();
            name.map(AppMessage::SaveTheme)
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ThemeSelectorModalState {
    pub theme_selector_state: ThemeSelectorState,
    pub visible: bool,
    original: Theme,
}

impl ThemeSelectorModalState {
    pub fn new(items: Vec<(String, Theme)>) -> Self {
        Self {
            theme_selector_state: ThemeSelectorState::new(items),
            visible: false,
            original: Theme::default(),
        }
    }

    fn open(&mut self, current: Theme) {
        self.original = current;
        self.theme_selector_state.select_theme(&current);
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn preview<'a>(&self) -> Option<AppMessage<'a>> {
        self.theme_selector_state
            .selected_theme()
            .map(AppMessage::PreviewTheme)
    }
}

pub struct ThemeSelectorModal<'a> {
    _lifetime: PhantomData<&'a ()>,
    pub border_type: BorderType,
    pub theme: Theme,
}

impl<'a> ThemeSelectorModal<'a> {
    pub fn new(border_type: BorderType, theme: Theme) -> Self {
        Self {
            _lifetime: PhantomData,
            border_type,
            theme,
        }
    }

    fn modal_area(&self, area: Rect) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(50)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Length(40)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }
}

impl<'a> StatefulWidget for ThemeSelectorModal<'a> {
    type State = ThemeSelectorModalState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = self.modal_area(area);
        Widget::render(Clear, area, buf);
        ThemeSelector::new(self.border_type, self.theme).render(
            area,
            buf,
            &mut state.theme_selector_state,
        );
    }
}
