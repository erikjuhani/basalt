use std::ops::Range;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, List, ListItem, StatefulWidget, Widget},
};

use crate::{
    config::Theme,
    search::search_state::{SearchResult, SearchState},
};

#[derive(Debug, Clone)]
pub struct SearchModal {
    border_type: BorderType,
    theme: Theme,
    prompt: String,
}

impl SearchModal {
    pub fn new(border_type: BorderType, theme: Theme, prompt: String) -> Self {
        Self {
            border_type,
            theme,
            prompt,
        }
    }

    fn modal_area(area: Rect) -> Rect {
        let [area] = Layout::vertical([Constraint::Percentage(70)])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::horizontal([Constraint::Length(80)])
            .flex(Flex::Center)
            .areas(area);
        area
    }

    fn render_input(&self, buf: &mut Buffer, area: Rect, state: &SearchState) {
        Line::from(vec![
            Span::styled(self.prompt.as_str(), Style::new().fg(self.theme.accent)),
            Span::styled(state.query.value(), Style::new().fg(self.theme.text)),
        ])
        .render(area, buf);

        let cursor_x = area.x + self.prompt.chars().count() as u16 + state.query.cursor() as u16;
        if cursor_x < area.right() {
            buf.set_style(
                Rect::new(cursor_x, area.y, 1, 1),
                Style::new().fg(self.theme.muted).reversed(),
            );
        }
    }

    fn row(&self, result: &SearchResult) -> ListItem<'static> {
        let mut spans = vec![Span::styled(
            format!("{}  ", result.note.name()),
            Style::new().fg(self.theme.muted),
        )];
        spans.extend(highlight(
            &result.text,
            &result.match_ranges,
            Style::new().fg(self.theme.text),
            // Colour the match by foreground, not an inverted chip: the selected
            // row patches the background, which would hide a background chip.
            Style::new().fg(self.theme.accent).bold(),
        ));
        ListItem::new(Line::from(spans))
    }
}

impl StatefulWidget for SearchModal {
    type State = SearchState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = Self::modal_area(area);
        Clear.render(area, buf);

        let status = if state.indexing() {
            "indexing…".to_string()
        } else {
            format!("{}/{}", state.results.len(), state.total)
        };

        let block = Block::bordered()
            .border_type(self.border_type)
            .fg(self.theme.muted)
            .bg(self.theme.background)
            .title(Line::from(" Search ").style(Style::new().italic().bold()))
            .title_bottom(Line::from(format!(" {status} ")).right_aligned());

        let inner = block.inner(area);
        block.render(area, buf);

        let [prompt_area, list_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);

        self.render_input(buf, prompt_area, state);

        if state.query.is_empty() {
            Line::from("Type to search notes")
                .style(Style::new().fg(self.theme.muted))
                .alignment(Alignment::Center)
                .render(list_area, buf);
            return;
        }

        let list = List::new(state.results.iter().map(|result| self.row(result)))
            .highlight_style(Style::new().bg(self.theme.code_bg))
            .highlight_symbol("▎ ");
        StatefulWidget::render(list, list_area, buf, &mut state.list_state);
    }
}

fn highlight(
    text: &str,
    ranges: &[Range<usize>],
    base: Style,
    matched: Style,
) -> Vec<Span<'static>> {
    let flagged: Vec<(char, bool)> = text
        .char_indices()
        .map(|(byte, character)| (character, ranges.iter().any(|range| range.contains(&byte))))
        .collect();

    flagged
        .chunk_by(|(_, a), (_, b)| a == b)
        .map(|run| {
            let text: String = run.iter().map(|&(character, _)| character).collect();
            Span::styled(text, if run[0].1 { matched } else { base })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::search_state::loaded_index;
    use basalt_core::obsidian::Note;

    #[test]
    fn test_search_render() {
        use insta::assert_snapshot;
        use ratatui::{backend::TestBackend, Terminal};

        let notes = vec![
            (
                Note::new_unchecked("Link notes", std::path::Path::new("/Link notes.md")),
                "1. Type \"three\" to find the first note you created.".to_string(),
            ),
            (
                Note::new_unchecked(
                    "No prior experience",
                    std::path::Path::new("/No prior experience.md"),
                ),
                "congratulations on finding Obsidian!".to_string(),
            ),
            (
                Note::new_unchecked(
                    "Vault is just a local folder",
                    std::path::Path::new("/Vault is just a local folder.md"),
                ),
                "open it with your system explorer or Finder, zip it up.".to_string(),
            ),
        ];

        let mut state = SearchState {
            visible: true,
            index: loaded_index(notes),
            ..SearchState::default()
        };
        "find"
            .chars()
            .for_each(|character| state.insert_char(character));

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal
            .draw(|frame| {
                SearchModal::new(BorderType::Rounded, Theme::default(), "> ".to_string()).render(
                    frame.area(),
                    frame.buffer_mut(),
                    &mut state,
                )
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
