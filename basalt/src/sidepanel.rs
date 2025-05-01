use std::marker::PhantomData;

use basalt_core::obsidian::Note;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, List, ListItem, ListState, StatefulWidgetRef},
};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SidePanelState<'a> {
    pub(crate) title: &'a str,
    pub(crate) selected_item_index: Option<usize>,
    pub(crate) items: Vec<Note>,
    pub(crate) open: bool,
    list_state: ListState,
}

impl<'a> SidePanelState<'a> {
    pub fn new(title: &'a str, items: Vec<Note>) -> Self {
        SidePanelState {
            items,
            title,
            selected_item_index: None,
            list_state: ListState::default().with_selected(Some(0)),
            open: true,
        }
    }

    pub fn open(self) -> Self {
        Self { open: true, ..self }
    }

    pub fn close(self) -> Self {
        Self {
            open: false,
            ..self
        }
    }

    pub fn toggle(self) -> Self {
        Self {
            open: !self.open,
            ..self
        }
    }

    fn calculate_offset(&self, window_height: usize) -> usize {
        if self.items.is_empty() {
            return 0;
        }

        let half = window_height / 2;

        let idx = self.list_state.selected().unwrap_or_default();

        // When the selected item is near the end of the list and there aren't enough items
        // remaining to keep the selection vertically centered, we shift the offset to show
        // as many trailing items as possible instead of centering the selection.
        //
        // This prevents empty lines from appearing at the bottom of the list when the
        // selection moves toward the end.
        //
        // Without this check, you'd see output like:
        // ╭────────╮
        // │ 3 item │
        // │>4 item │
        // │ 5 item │
        // │        │
        // ╰────────╯
        //
        // With this check, the list scrolls up to fill the remaining space:
        // ╭────────╮
        // │ 2 item │
        // │ 3 item │
        // │>4 item │
        // │ 5 item │
        // ╰────────╯
        //
        // The goal is to avoid showing unnecessary blank rows and to maximize visible items.
        if idx + half > self.items.len() - 1 {
            self.items.len().saturating_sub(window_height)
        } else {
            idx.saturating_sub(half)
        }
    }

    pub fn update_offset_mut(&mut self, window_height: usize) -> &Self {
        let offset = self.calculate_offset(window_height);

        let list_state = &mut self.list_state;
        *list_state.offset_mut() = offset;

        self
    }

    pub fn select(&self) -> Self {
        Self {
            selected_item_index: self.list_state.selected(),
            ..self.clone()
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected_item_index
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn next(mut self) -> Self {
        let index = self
            .list_state
            .selected()
            .map(|i| (i + 1).min(self.items.len() - 1));

        self.list_state.select(index);

        Self {
            list_state: self.list_state,
            ..self
        }
    }

    pub fn previous(mut self) -> Self {
        self.list_state.select_previous();

        Self {
            list_state: self.list_state,
            ..self
        }
    }
}

#[derive(Default)]
pub struct SidePanel<'a> {
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> StatefulWidgetRef for SidePanel<'a> {
    type State = SidePanelState<'a>;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title_style(Style::default().italic().bold());

        let items: Vec<ListItem> = state
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| match state.selected() {
                Some(selected) if selected == i => ListItem::new(if state.open {
                    format!("◆ {}", item.name)
                } else {
                    "◆".to_string()
                }),
                _ if state.open => ListItem::new(format!("  {}", item.name)),
                _ => ListItem::new("◦"),
            })
            .collect();

        let inner_area = block.inner(area);

        state.update_offset_mut(inner_area.height.into());

        if state.open {
            List::new(items.to_vec())
                .block(
                    block
                        .title(format!(" {} ", state.title))
                        .title(Line::from(" ◀ ").alignment(Alignment::Right)),
                )
                .highlight_style(Style::new().reversed().dark_gray())
                .highlight_symbol(" ")
                .render_ref(area, buf, &mut state.list_state);
        } else {
            let layout = Layout::horizontal([Constraint::Length(5)]).split(area);

            List::new(items)
                .block(block.title(" ▶ "))
                .highlight_style(Style::new().reversed().dark_gray())
                .highlight_symbol(" ")
                .render_ref(layout[0], buf, &mut state.list_state);
        }
    }
}
