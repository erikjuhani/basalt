use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Padding, Paragraph, StatefulWidget, Widget},
};

use crate::config::Theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PendingAction {
    Quit,
    CloseTab,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Save,
    Discard,
    Cancel,
}

pub fn handle_key_event(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('s' | 'S') | KeyCode::Enter => Some(Message::Save),
        KeyCode::Char('d' | 'D') => Some(Message::Discard),
        KeyCode::Char('c' | 'C') | KeyCode::Esc => Some(Message::Cancel),
        _ => None,
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct UnsavedModalState {
    pending: Option<(PendingAction, usize)>,
}

impl UnsavedModalState {
    pub fn is_visible(&self) -> bool {
        self.pending.is_some()
    }

    pub fn open(&mut self, action: PendingAction, count: usize) {
        self.pending = Some((action, count));
    }

    pub fn take(&mut self) -> Option<PendingAction> {
        self.pending.take().map(|(action, _)| action)
    }
}

fn modal_area(area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(8)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(52)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

pub struct UnsavedModal {
    pub border_type: BorderType,
    pub theme: Theme,
}

impl UnsavedModal {
    pub fn new(border_type: BorderType, theme: Theme) -> Self {
        Self { border_type, theme }
    }

    fn key_hint<'a>(&self, key: &'a str, label: &'a str) -> [Span<'a>; 2] {
        [
            Span::from(key).fg(self.theme.accent).bold(),
            Span::from(label).fg(self.theme.text),
        ]
    }
}

impl StatefulWidget for UnsavedModal {
    type State = UnsavedModalState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let Some((action, count)) = state.pending else {
            return;
        };

        let notes = if count == 1 { "note" } else { "notes" };
        let question = match action {
            PendingAction::Quit => "Save before quitting?",
            PendingAction::CloseTab => "Save before closing the tab?",
        };

        let block = Block::bordered()
            .fg(self.theme.muted)
            .bg(self.theme.background)
            .border_type(self.border_type)
            .padding(Padding::uniform(1))
            .title_style(Style::default().italic().bold())
            .title(" Unsaved changes ");

        let hints: Vec<Span> = [
            self.key_hint("s", " save   "),
            self.key_hint("d", " discard   "),
            self.key_hint("esc", " cancel"),
        ]
        .into_iter()
        .flatten()
        .collect();

        let text = vec![
            Line::from(format!("Unsaved changes in {count} {notes}.")).fg(self.theme.text),
            Line::from(question).fg(self.theme.text),
            Line::default(),
            Line::from(hints),
        ];

        let area = modal_area(area);
        Widget::render(Clear, area, buf);
        Paragraph::new(text).block(block).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, crossterm::event::KeyModifiers, Terminal};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn keys_map_to_choices() {
        let cases = [
            (KeyCode::Char('s'), Some(Message::Save)),
            (KeyCode::Enter, Some(Message::Save)),
            (KeyCode::Char('d'), Some(Message::Discard)),
            (KeyCode::Char('c'), Some(Message::Cancel)),
            (KeyCode::Char('n'), None),
            (KeyCode::Char('y'), None),
            (KeyCode::Esc, Some(Message::Cancel)),
            (KeyCode::Char('q'), None),
            (KeyCode::Char('w'), None),
        ];

        for (code, expected) in cases {
            assert_eq!(handle_key_event(key(code)), expected, "{code:?}");
        }
    }

    #[test]
    fn take_closes_the_modal() {
        let mut state = UnsavedModalState::default();
        assert!(!state.is_visible());

        state.open(PendingAction::Quit, 2);
        assert!(state.is_visible());

        assert_eq!(state.take(), Some(PendingAction::Quit));
        assert!(!state.is_visible());
        assert_eq!(state.take(), None);
    }

    #[test]
    fn render() {
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();

        for (name, action, count) in [
            ("quit", PendingAction::Quit, 3),
            ("close_tab", PendingAction::CloseTab, 1),
        ] {
            let mut state = UnsavedModalState::default();
            state.open(action, count);
            terminal
                .draw(|frame| {
                    UnsavedModal::new(BorderType::Rounded, Theme::default()).render(
                        frame.area(),
                        frame.buffer_mut(),
                        &mut state,
                    )
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        }
    }
}
