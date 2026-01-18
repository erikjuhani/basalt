use std::marker::PhantomData;

use basalt_core::obsidian::Vault;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, List, ListItem, ListState, StatefulWidget},
};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VaultSelectorState<'a> {
    pub(crate) selected_item_index: Option<usize>,
    pub(crate) items: Vec<&'a Vault>,
    list_state: ListState,
}

impl<'a> VaultSelectorState<'a> {
    pub fn new(items: Vec<&'a Vault>) -> Self {
        VaultSelectorState {
            items,
            selected_item_index: None,
            list_state: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn select(&mut self) {
        self.selected_item_index = self.list_state.selected();
    }

    pub fn items(self) -> Vec<&'a Vault> {
        self.items
    }

    pub fn get_item(self, index: usize) -> Option<&'a Vault> {
        self.items.get(index).cloned()
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected_item_index
    }

    pub fn next(&mut self) {
        let index = self
            .list_state
            .selected()
            .map(|i| (i + 1).min(self.items.len() - 1));

        self.list_state.select(index);
    }

    pub fn previous(&mut self) {
        self.list_state.select_previous();
    }
}

#[derive(Default)]
pub struct VaultSelector<'a> {
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> StatefulWidget for VaultSelector<'a> {
    type State = VaultSelectorState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let items: Vec<ListItem> = state
            .items
            .iter()
            .map(|item| {
                if item.open {
                    ListItem::new(format!("◆ {}", item.name))
                } else {
                    ListItem::new(format!("  {}", item.name))
                }
            })
            .collect();

        List::new(items)
            .block(
                Block::bordered()
                    .dark_gray()
                    .title(" Vaults ")
                    .title_style(Style::default().italic().bold())
                    .border_type(BorderType::Rounded),
            )
            .fg(Color::default())
            .highlight_style(Style::new().reversed().dark_gray())
            .highlight_symbol(" ")
            .render(area, buf, &mut state.list_state);
    }
}
