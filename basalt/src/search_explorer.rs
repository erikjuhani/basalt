use basalt_core::obsidian::{Note, VaultEntry};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32Str,
};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Rect, Size},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

use crate::app::{
    calc_scroll_amount, ActivePane, Message as AppMessage, ScrollAmount, SelectedNote,
};

#[derive(Debug, Default, Clone, PartialEq)]
enum Mode {
    #[default]
    Searching,
    Navigating,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SearchExplorerState {
    query: String,
    cursor_col: usize,
    mode: Mode,
    results: Vec<(Note, u32)>,
    all_notes: Vec<Note>,
    pub(crate) list_state: ListState,
    pub(crate) visible: bool,
}

impl SearchExplorerState {
    pub fn open(&mut self, entries: &[VaultEntry]) {
        self.entries = entries.flatten();
        self.query.clear();
        self.cursor_col = 0;
        self.mode = Mode::Searching;
        self.results.clear();
        self.list_state = ListState::default();
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.query.clear();
        self.cursor_col = 0;
        self.mode = Mode::Searching;
        self.results.clear();
        self.all_notes.clear();
        self.list_state = ListState::default();
        self.visible = false;
    }

    pub fn is_searching(&self) -> bool {
        matches!(self.mode, Mode::Searching)
    }

    fn set_navigating(&mut self) {
        self.mode = Mode::Navigating;
        if self.list_state.selected().is_none() && !self.results.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    fn byte_index(&self) -> usize {
        self.query
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.cursor_col)
            .unwrap_or(self.query.len())
    }

    pub fn insert_char(&mut self, c: char) {
        self.query.insert(self.byte_index(), c);
        self.cursor_col += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_col == 0 {
            return;
        }

        if let Some((byte_index, _)) = self.query.char_indices().nth(self.cursor_col - 1) {
            self.query.remove(byte_index);
            self.cursor_col = self.cursor_col.saturating_sub(1);
        }
    }

    fn cursor_left(&mut self) {
        self.cursor_col = self.cursor_col.saturating_sub(1);
    }

    fn cursor_right(&mut self) {
        self.cursor_col = self
            .cursor_col
            .saturating_add(1)
            .min(self.query.chars().count());
    }

    fn update_results(&mut self) {
        if self.query.is_empty() {
            self.results.clear();
            self.list_state.select(None);
            return;
        }

        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::parse(&self.query, CaseMatching::Ignore, Normalization::Smart);

        let mut scored = Vec::new();
        let mut buf = Vec::new();
        for note in &self.all_notes {
            buf.clear();
            let haystack = Utf32Str::new(note.name(), &mut buf);
            if let Some(score) = pattern.score(haystack, &mut matcher) {
                scored.push((note.clone(), score));
            }
        }

        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.name().cmp(b.0.name())));
        self.results = scored;

        if self.results.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn selected_note(&self) -> Option<Note> {
        let idx = self.list_state.selected()?;
        self.results.get(idx).map(|(note, _)| note.clone())
    }

    fn next(&mut self, amount: usize) {
        let index = self
            .list_state
            .selected()
            .map(|i| (i + amount).min(self.results.len().saturating_sub(1)));
        self.list_state.select(index);
    }

    fn previous(&mut self, amount: usize) {
        let index = self.list_state.selected().map(|i| i.saturating_sub(amount));
        self.list_state.select(index);
    }
}

pub(crate) fn collect_all_notes(entries: &[VaultEntry]) -> Vec<Note> {
    entries
        .iter()
        .flat_map(|entry| match entry {
            VaultEntry::File(note) => vec![note.clone()],
            VaultEntry::Directory { entries, .. } => collect_all_notes(entries),
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Up,
    Down,
    Open,
    Close,
    SearchMode,
    ScrollUp(ScrollAmount),
    ScrollDown(ScrollAmount),
    KeyEvent(KeyEvent),
}

pub fn update<'a>(
    message: &Message,
    screen_size: Size,
    state: &mut SearchExplorerState,
) -> Option<AppMessage<'a>> {
    match message {
        Message::Up => state.previous(1),
        Message::Down => state.next(1),
        Message::Open => {
            if let Some(note) = state.selected_note() {
                state.close();
                return Some(AppMessage::Batch(vec![
                    AppMessage::SelectNote(SelectedNote::from(&note)),
                    AppMessage::SetActivePane(ActivePane::Explorer),
                ]));
            }
        }
        Message::Close => {
            state.close();
            return Some(AppMessage::SetActivePane(ActivePane::Explorer));
        }
        Message::SearchMode => {
            state.mode = Mode::Searching;
        }
        Message::ScrollUp(scroll_amount) => {
            state.previous(calc_scroll_amount(scroll_amount, screen_size.height.into()));
        }
        Message::ScrollDown(scroll_amount) => {
            state.next(calc_scroll_amount(scroll_amount, screen_size.height.into()));
        }
        Message::KeyEvent(key) => match key.code {
            KeyCode::Char(c) => {
                state.insert_char(c);
                state.update_results();
            }
            KeyCode::Backspace => {
                state.delete_char();
                state.update_results();
            }
            KeyCode::Left => state.cursor_left(),
            KeyCode::Right => state.cursor_right(),
            KeyCode::Esc => {
                if state.query.is_empty() {
                    state.close();
                    return Some(AppMessage::SetActivePane(ActivePane::Explorer));
                }
                state.set_navigating();
            }
            KeyCode::Enter => {
                if let Some(note) = state.selected_note() {
                    state.close();
                    return Some(AppMessage::Batch(vec![
                        AppMessage::SelectNote(SelectedNote::from(&note)),
                        AppMessage::SetActivePane(ActivePane::Explorer),
                    ]));
                }
            }
            KeyCode::Down => state.next(1),
            KeyCode::Up => state.previous(1),
            _ => {}
        },
    }

    None
}

pub fn handle_searching_event(key: &KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::ALT) => None,
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::ALT) => None,
        _ => Some(Message::KeyEvent(*key)),
    }
}

#[derive(Clone, Debug, Default)]
pub struct SearchExplorer;

impl StatefulWidget for SearchExplorer {
    type State = SearchExplorerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Search ")
            .title_style(Style::default().italic().bold());

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.is_empty() {
            return;
        }

        let [search_area, separator_area, list_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(inner);

        let search_line = if state.query.is_empty() && state.is_searching() {
            Line::from(vec![
                Span::raw(" "),
                Span::raw("Type to search...").dark_gray().italic(),
            ])
        } else {
            Line::from(vec![Span::raw(" "), Span::raw(state.query.as_str())])
        };

        Paragraph::new(search_line).render(search_area, buf);

        if state.is_searching() {
            let cursor_x = search_area.x.saturating_add(state.cursor_col as u16 + 1);
            if cursor_x < search_area.right() {
                buf.set_style(
                    Rect::new(cursor_x, search_area.y, 1, 1),
                    Style::default().reversed(),
                );
            }
        }

        if separator_area.height > 0 {
            let style = Style::default().dark_gray();
            if inner.width > 0 {
                let middle = "─".repeat(inner.width as usize);
                buf.set_string(inner.x, separator_area.y, middle, style);
            }
            buf.set_string(area.x, separator_area.y, "├", style);
            buf.set_string(area.right().saturating_sub(1), separator_area.y, "┤", style);
        }

        if !list_area.is_empty() {
            if !state.query.is_empty() && state.results.is_empty() {
                Paragraph::new(Line::from(" No results".dark_gray().italic()))
                    .render(list_area, buf);
                return;
            }

            let items: Vec<ListItem> = state
                .results
                .iter()
                .map(|(note, _)| ListItem::new(Line::from(vec!["  ".into(), note.name().into()])))
                .collect();

            StatefulWidget::render(
                List::new(items)
                    .highlight_style(Style::new().reversed().dark_gray())
                    .highlight_symbol(" "),
                list_area,
                buf,
                &mut state.list_state,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use basalt_core::obsidian::Note;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    fn sample_notes() -> Vec<Note> {
        vec![
            Note::new_unchecked("Index", Path::new("Index.md")),
            Note::new_unchecked("Induction Tape", Path::new("Induction Tape.md")),
            Note::new_unchecked("Basalt Guide", Path::new("Basalt Guide.md")),
            Note::new_unchecked("Indigo", Path::new("Indigo.md")),
            Note::new_unchecked("Meeting Notes", Path::new("Meeting Notes.md")),
        ]
    }

    #[test]
    fn test_search_explorer_states() {
        type TestCase = (&'static str, Box<dyn Fn() -> SearchExplorerState>);

        let tests: Vec<TestCase> = vec![
            (
                "empty_search",
                Box::new(|| {
                    let mut state = SearchExplorerState::default();
                    state.open(sample_notes());
                    state
                }),
            ),
            (
                "with_query",
                Box::new(|| {
                    let mut state = SearchExplorerState::default();
                    state.open(sample_notes());
                    state.insert_char('I');
                    state.insert_char('n');
                    state.insert_char('d');
                    state.update_results();
                    state
                }),
            ),
            (
                "no_results",
                Box::new(|| {
                    let mut state = SearchExplorerState::default();
                    state.open(sample_notes());
                    state.insert_char('z');
                    state.insert_char('z');
                    state.insert_char('z');
                    state.update_results();
                    state
                }),
            ),
            (
                "navigate_mode",
                Box::new(|| {
                    let mut state = SearchExplorerState::default();
                    state.open(sample_notes());
                    state.insert_char('I');
                    state.insert_char('n');
                    state.update_results();
                    state.set_navigating();
                    state.next(1);
                    state
                }),
            ),
        ];

        let mut terminal = Terminal::new(TestBackend::new(35, 10)).unwrap();

        tests.into_iter().for_each(|(name, state_fn)| {
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    let mut state = state_fn();
                    SearchExplorer.render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }
}
