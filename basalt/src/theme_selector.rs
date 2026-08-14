use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    widgets::{
        Block, BorderType, List, ListItem, ListState, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget,
    },
};

use crate::config::Theme;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ThemeSelectorState {
    pub(crate) items: Vec<(String, Theme)>,
    list_state: ListState,
}

impl ThemeSelectorState {
    pub fn new(items: Vec<(String, Theme)>) -> Self {
        Self {
            items,
            list_state: ListState::default().with_selected(Some(0)),
        }
    }

    /// Highlights the entry matching `theme`, leaving the selection unchanged
    /// when none matches.
    pub fn select_theme(&mut self, theme: &Theme) {
        if let Some(index) = self.items.iter().position(|(_, item)| item == theme) {
            self.list_state.select(Some(index));
        }
    }

    pub fn selected_theme(&self) -> Option<Theme> {
        self.list_state
            .selected()
            .and_then(|index| self.items.get(index))
            .map(|(_, theme)| *theme)
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.list_state
            .selected()
            .and_then(|index| self.items.get(index))
            .map(|(name, _)| name.as_str())
    }

    pub fn next(&mut self) {
        let index = self
            .list_state
            .selected()
            .map(|i| (i + 1).min(self.items.len().saturating_sub(1)));
        self.list_state.select(index);
    }

    pub fn previous(&mut self) {
        self.list_state.select_previous();
    }
}

pub struct ThemeSelector {
    pub border_type: BorderType,
    pub theme: Theme,
}

impl ThemeSelector {
    pub fn new(border_type: BorderType, theme: Theme) -> Self {
        Self { border_type, theme }
    }
}

impl StatefulWidget for ThemeSelector {
    type State = ThemeSelectorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let items: Vec<ListItem> = state
            .items
            .iter()
            .map(|(name, _)| ListItem::new(format!("  {name}")))
            .collect();

        let items_count = items.len();

        List::new(items)
            .block(
                Block::bordered()
                    .fg(self.theme.muted)
                    .bg(self.theme.background)
                    .title(" Themes ")
                    .title_style(Style::default().italic().bold())
                    .border_type(self.border_type),
            )
            .fg(self.theme.text)
            .highlight_style(Style::new().reversed().fg(self.theme.muted))
            .highlight_symbol(" ")
            .render(area, buf, &mut state.list_state);

        // Minimum amount of items that can be rendered without the scrollbar.
        let min_item_amount = 4;

        if !area.is_empty() && items_count > min_item_amount {
            let mut scroll_state =
                ScrollbarState::new(items_count).position(state.list_state.selected().unwrap_or(0));

            Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                area,
                buf,
                &mut scroll_state,
            );
        }
    }
}
