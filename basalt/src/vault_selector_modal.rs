use std::marker::PhantomData;

use basalt_core::obsidian::Vault;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    widgets::{Clear, ScrollbarState, StatefulWidget, StatefulWidgetRef, Widget},
};

use crate::vault_selector::{VaultSelector, VaultSelectorState};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VaultSelectorModalState<'a> {
    pub vault_selector_state: VaultSelectorState<'a>,
    pub visible: bool,
}

impl<'a> VaultSelectorModalState<'a> {
    pub fn new(items: Vec<&'a Vault>) -> Self {
        Self {
            vault_selector_state: VaultSelectorState::new(items),
            visible: false,
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.vault_selector_state.selected()
    }

    pub fn select(&self) -> Self {
        Self {
            vault_selector_state: self.vault_selector_state.clone().select(),
            ..self.clone()
        }
    }

    pub fn get_item(self, index: usize) -> Option<&'a Vault> {
        self.vault_selector_state.get_item(index)
    }

    pub fn next(&self) -> Self {
        Self {
            vault_selector_state: self.vault_selector_state.clone().next(),
            ..self.clone()
        }
    }

    pub fn previous(&self) -> Self {
        Self {
            vault_selector_state: self.vault_selector_state.clone().previous(),
            ..self.clone()
        }
    }

    pub fn hide(&self) -> Self {
        Self {
            visible: false,
            ..self.clone()
        }
    }

    pub fn toggle_visibility(&self) -> Self {
        Self {
            visible: !self.visible,
            ..self.clone()
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VaultSelectorModal<'a> {
    _lifetime: PhantomData<&'a ()>,
}

impl VaultSelectorModal<'_> {
    fn modal_area(self, area: Rect) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(50)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Length(60)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }
}

impl<'a> StatefulWidget for VaultSelectorModal<'a> {
    type State = VaultSelectorModalState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let area = self.modal_area(area);
        Widget::render(Clear, area, buf);
        VaultSelector::default().render_ref(area, buf, &mut state.vault_selector_state);
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ModalTitle<'a> {
    pub left: &'a str,
    pub right: Option<&'a str>,
}

impl<'a> ModalTitle<'a> {
    pub fn new(title_left: &'a str, title_right: Option<&'a str>) -> Self {
        Self {
            left: title_left,
            right: title_right,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ModalState<'a> {
    pub scrollbar_state: ScrollbarState,
    pub scrollbar_position: usize,
    pub viewport_height: usize,
    pub text: &'a str,
    pub title: ModalTitle<'a>,
    pub is_open: bool,
}

impl<'a> ModalState<'a> {
    pub fn new(title: ModalTitle<'a>, text: &'a str) -> Self {
        Self {
            title,
            text,
            scrollbar_state: ScrollbarState::new(text.lines().count()),
            ..Default::default()
        }
    }

    pub fn scroll_up(self, amount: usize) -> Self {
        let scrollbar_position = self.scrollbar_position.saturating_sub(amount);
        let scrollbar_state = self.scrollbar_state.position(scrollbar_position);

        Self {
            scrollbar_state,
            scrollbar_position,
            ..self
        }
    }

    pub fn scroll_down(self, amount: usize) -> Self {
        let scrollbar_position = self
            .scrollbar_position
            .saturating_add(amount)
            .min(self.text.lines().count());

        let scrollbar_state = self.scrollbar_state.position(scrollbar_position);

        Self {
            scrollbar_state,
            scrollbar_position,
            ..self
        }
    }

    pub fn reset_scrollbar(self) -> Self {
        Self {
            scrollbar_state: ScrollbarState::default(),
            scrollbar_position: 0,
            ..self
        }
    }
}
