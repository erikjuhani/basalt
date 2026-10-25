use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Clear, List, ListItem, StatefulWidget, Widget},
};

use crate::{
    completion::state::{CompletionState, Suggestion},
    config::Theme,
    search::highlight,
};

const MAX_ROWS: u16 = 8;
const MIN_WIDTH: u16 = 24;
const MAX_WIDTH: u16 = 48;

#[derive(Debug, Clone)]
pub struct Dropdown {
    border_type: BorderType,
    theme: Theme,
    cursor: Position,
}

impl Dropdown {
    pub fn new(border_type: BorderType, theme: Theme, cursor: Position) -> Self {
        Self {
            border_type,
            theme,
            cursor,
        }
    }

    fn dropdown_area(&self, area: Rect, state: &CompletionState) -> Rect {
        let rows = (state.results.len() as u16).clamp(1, MAX_ROWS);
        let widest = state
            .results
            .iter()
            .map(|suggestion| suggestion.label.chars().count() as u16)
            .max()
            .unwrap_or_default();
        let width = (widest + 4).clamp(MIN_WIDTH, MAX_WIDTH).min(area.width);
        let height = (rows + 2).min(area.height);

        let below = self.cursor.y.saturating_add(1);
        let y = if below + height <= area.bottom() {
            below
        } else {
            self.cursor.y.saturating_sub(height).max(area.y)
        };
        let x = self
            .cursor
            .x
            .min(area.right().saturating_sub(width))
            .max(area.x);

        Rect::new(x, y, width, height)
    }

    fn row(&self, suggestion: &Suggestion) -> ListItem<'static> {
        ListItem::new(Line::from(highlight(
            &suggestion.label,
            &suggestion.match_ranges,
            Style::new().fg(self.theme.text),
            Style::new().fg(self.theme.accent).bold(),
        )))
    }
}

impl StatefulWidget for Dropdown {
    type State = CompletionState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = self.dropdown_area(area, state);
        Clear.render(area, buf);

        let block = Block::bordered()
            .border_type(self.border_type)
            .fg(self.theme.muted)
            .bg(self.theme.background);
        let inner = block.inner(area);
        block.render(area, buf);

        if state.results.is_empty() {
            Line::from("No matches")
                .style(Style::new().fg(self.theme.muted).italic())
                .render(inner, buf);
            return;
        }

        let list = List::new(state.results.iter().map(|suggestion| self.row(suggestion)))
            .highlight_style(Style::new().bg(self.theme.code_bg))
            .highlight_symbol("▎");
        StatefulWidget::render(list, inner, buf, &mut state.list_state);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use basalt_core::obsidian::Note;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    use super::*;

    fn render(cursor: Position) -> Terminal<TestBackend> {
        let mut state = CompletionState::default();
        state.open(
            0,
            Path::new("/vault/test.md"),
            ["Daily log", "Project ideas", "Reading list"]
                .into_iter()
                .map(|name| Note::new_unchecked(name, Path::new(&format!("/vault/{name}.md"))))
                .collect(),
        );

        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        terminal
            .draw(|frame| {
                Dropdown::new(BorderType::Rounded, Theme::default(), cursor).render(
                    frame.area(),
                    frame.buffer_mut(),
                    &mut state,
                )
            })
            .unwrap();
        terminal
    }

    #[test]
    fn test_dropdown_below_cursor() {
        assert_snapshot!(render(Position::new(4, 1)).backend());
    }

    #[test]
    fn test_dropdown_flips_above_cursor_near_bottom() {
        assert_snapshot!(render(Position::new(30, 8)).backend());
    }
}
