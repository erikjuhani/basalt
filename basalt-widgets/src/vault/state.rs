use basalt_core::obsidian::Vault;
use ratatui::widgets::{ListState, ScrollbarState};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VaultSelectorState<'a> {
    pub items: Vec<&'a Vault>,
    pub list_state: ListState,
    pub selected_item_index: Option<usize>,
    pub viewport_height: usize,
    pub scrollbar_state: ScrollbarState,
    pub scrollbar_position: usize,
    pub is_modal: bool,
}

impl<'a> VaultSelectorState<'a> {
    pub fn new(items: Vec<&'a Vault>) -> Self {
        VaultSelectorState {
            items,
            list_state: ListState::default().with_selected(Some(0)),
            selected_item_index: None,
            viewport_height: 0,
            scrollbar_state: ScrollbarState::new(0),
            scrollbar_position: 0,
            is_modal: false,
        }
    }

    pub fn select(&mut self) {
        self.selected_item_index = self.list_state.selected();
    }

    pub fn items(self) -> Vec<&'a Vault> {
        self.items
    }

    pub fn get_item(&self, index: usize) -> Option<&'a Vault> {
        self.items.get(index).cloned()
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected_item_index
    }

    pub fn next(&mut self) {
        let index = self
            .list_state
            .selected()
            .map(|i| i.saturating_add(1).min(self.items.len().saturating_sub(1)));

        self.list_state.select(index);
    }

    pub fn previous(&mut self) {
        let index = self.list_state.selected().map(|i| i.saturating_sub(1));

        self.list_state.select(index);
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
            .min(self.items.len());

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
