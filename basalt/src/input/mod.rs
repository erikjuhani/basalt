use basalt_core::obsidian::{directory::Directory, rename_dir, rename_note, Note};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Offset, Position, Rect},
    style::{Color, Style, Stylize},
    text::Span,
    widgets::{Block, BorderType, Clear, Padding, Paragraph, StatefulWidget, Widget},
};

use crate::app::{ActivePane, Message as AppMessage};

#[derive(Clone, Default, Debug, PartialEq)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Callback {
    RenameDir(Directory),
    RenameNote(Note),
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct InputModalState {
    input: String,
    input_original: String,
    cursor_col: usize,
    cursor_row: usize,
    input_mode: InputMode,
    scroll: usize,
    modified: bool,
    visible: bool,
    label: String,
    offset_x: usize,
    callback: Option<Callback>,
}

impl InputModalState {
    pub fn new(value: &str, row: usize, visible: bool) -> Self {
        Self {
            input: value.to_string(),
            input_original: value.to_string(),
            cursor_col: value.chars().count(),
            cursor_row: row,
            input_mode: InputMode::Editing,
            scroll: 0,
            offset_x: 0,
            modified: false,
            visible,
            label: String::from("Input"),
            callback: None,
        }
    }

    pub fn set_input(&mut self, value: &str) {
        self.input = value.to_string();
        self.input_original = value.to_string();
        self.scroll = 0;
        self.cursor_col = value.chars().count();
        self.input_mode = InputMode::Editing;
    }

    pub fn set_label(&mut self, label: &str) {
        self.label = label.to_string();
    }

    pub fn set_row(&mut self, row: usize) {
        self.cursor_row = row;
    }

    pub fn set_offset_x(&mut self, x: usize) {
        self.offset_x = x;
    }

    pub fn set_callback(&mut self, callback: &Callback) {
        self.callback = Some(callback.clone());
    }

    pub fn run_callback(&mut self) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        let result = if let Some(callback) = &self.callback {
            // FIXME: Propagate errors
            match callback {
                Callback::RenameNote(note) => {
                    let original_path = note.path().to_path_buf();
                    rename_note(note.clone(), &self.input)
                        .ok()
                        .map(|n| (original_path, n.path().to_path_buf()))
                }
                Callback::RenameDir(directory) => {
                    let original_path = directory.path().to_path_buf();
                    rename_dir(directory.clone(), &self.input)
                        .ok()
                        .map(|d| (original_path, d.path().to_path_buf()))
                }
            }
        } else {
            None
        };

        self.callback = None;
        result
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_editing(&self) -> bool {
        matches!(self.input_mode, InputMode::Editing)
    }

    fn cursor_left(&mut self, amount: usize) {
        let new_cursor_pos = self.cursor_col.saturating_sub(amount);
        self.cursor_col = self.clamp_cursor(new_cursor_pos);
    }

    fn cursor_word_backward(&mut self) {
        let remainder = &self.input[..self.byte_index()];

        let offset = remainder
            .chars()
            .rev()
            .skip_while(|c| c == &' ')
            .skip_while(|c| c != &' ')
            .count();

        self.cursor_col -= remainder.chars().count() - offset;
    }

    fn cursor_word_forward(&mut self) {
        let remainder = &self.input[self.byte_index()..];

        let offset = remainder
            .chars()
            .skip_while(|c| c != &' ')
            .skip_while(|c| c == &' ')
            .count();

        self.cursor_col += remainder.chars().count() - offset;
    }

    fn cursor_right(&mut self, amount: usize) {
        let new_cursor_pos = self.cursor_col.saturating_add(amount);
        self.cursor_col = self.clamp_cursor(new_cursor_pos);
    }

    pub fn insert_char(&mut self, char: char) {
        let index = self.byte_index();
        self.input.insert(index, char);
        self.modified = self.input != self.input_original;
        self.cursor_right(1);
    }

    pub fn delete_char(&mut self) {
        let index = self.byte_index();
        if index == 0 {
            return;
        }

        if let Some((byte_index, _)) = self.input.char_indices().nth(self.cursor_col - 1) {
            self.input.remove(byte_index);
            self.modified = self.input != self.input_original;
            self.cursor_left(1);
        }
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.cursor_col)
            .unwrap_or(self.input.len())
    }

    fn clamp_cursor(&self, cursor_pos: usize) -> usize {
        cursor_pos.clamp(0, self.input.chars().count())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputModalConfig {
    pub position: Position,
    pub label: String,
    pub initial_input: String,
    pub callback: Callback,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    CursorLeft,
    CursorRight,
    CursorWordForward,
    CursorWordBackward,
    Open(InputModalConfig),
    Accept,
    Delete,
    KeyEvent(KeyEvent),
    Cancel,
    EditMode,
}

pub fn update<'a>(message: &Message, state: &mut InputModalState) -> Option<AppMessage<'a>> {
    match message {
        Message::CursorLeft => {
            state.cursor_left(1);
        }
        Message::CursorRight => {
            state.cursor_right(1);
        }
        Message::CursorWordForward => {
            state.cursor_word_forward();
        }
        Message::CursorWordBackward => {
            state.cursor_word_backward();
        }
        Message::Cancel => match state.input_mode {
            InputMode::Editing => state.input_mode = InputMode::Normal,
            InputMode::Normal => {
                state.toggle_visibility();
                state.modified = false;
                return Some(AppMessage::SetActivePane(ActivePane::Explorer));
            }
        },
        Message::EditMode => {
            state.input_mode = InputMode::Editing;
        }
        Message::KeyEvent(key) => match key.code {
            KeyCode::Char(c) => {
                state.insert_char(c);
            }
            KeyCode::Enter => {
                if state.modified {
                    let rename = state.run_callback();
                    state.input_mode = InputMode::Normal;
                    state.toggle_visibility();
                    state.modified = false;
                    return Some(AppMessage::RefreshVault(rename));
                } else {
                    state.input_mode = InputMode::Normal;
                    return Some(AppMessage::Input(Message::Cancel));
                }
            }
            _ => {}
        },
        Message::Open(InputModalConfig {
            position,
            label,
            initial_input,
            callback,
        }) => {
            state.set_input(initial_input);
            state.set_row(position.y as usize);
            state.set_offset_x(position.x as usize);
            state.set_label(label);
            state.set_callback(callback);
            state.toggle_visibility();
            return Some(AppMessage::SetActivePane(ActivePane::Input));
        }
        Message::Delete => state.delete_char(),
        _ => {}
    }

    None
}

pub fn handle_editing_event(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::ALT) => {
            Some(Message::CursorWordForward)
        }
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::ALT) => {
            Some(Message::CursorWordBackward)
        }
        KeyCode::Left => Some(Message::CursorLeft),
        KeyCode::Right => Some(Message::CursorRight),
        KeyCode::Esc => Some(Message::Cancel),
        KeyCode::Backspace => Some(Message::Delete),
        _ => Some(Message::KeyEvent(key)),
    }
}

#[derive(Clone, Debug, Default)]
pub struct Input;

impl StatefulWidget for Input {
    type State = InputModalState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.visible {
            return;
        }

        // Input widget height is set to 3 since we include the borders
        let height = 3;

        let width = 40.min(area.width);

        let row = state.cursor_row;

        // let area = area.offset(self.offset);
        let y = if area.bottom() <= (row + height) as u16 {
            // We add 1 to go past the original line so it is still visible.
            (row - (height + 1)) as i32
        } else {
            row as i32
        };

        let area = area.offset(Offset {
            x: state.offset_x as i32,
            y,
        });

        let vertical = Layout::vertical([Constraint::Length(height as u16)]);
        let horizontal =
            Layout::horizontal([Constraint::Length(width + state.offset_x as u16 * 2)]);
        let [area] = vertical.areas::<1>(area);
        let [area] = horizontal.areas::<1>(area);

        Clear.render(area, buf);

        let row = y as u16;
        let col = state.cursor_col as u16 + area.left();

        if state.cursor_col > state.scroll + width as usize {
            state.scroll = state.cursor_col.saturating_sub(width as usize);
        } else if state.cursor_col < state.scroll {
            state.scroll = state.cursor_col;
        }

        let input = &state.input[state.scroll..];

        let mode_color = match state.input_mode {
            InputMode::Editing => Color::Green,
            InputMode::Normal => Color::Red,
        };

        let mode = format!("{:?}", state.input_mode)
            .fg(mode_color)
            .bold()
            .italic();

        let edited_marker = if state.modified {
            "*".bold().italic()
        } else {
            "".into()
        };

        Paragraph::new(input)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().dark_gray())
                    // TODO: Use a label field from state
                    .title(vec![
                        Span::from(" "),
                        Span::from(&state.label),
                        Span::from(": "),
                    ])
                    .padding(Padding::horizontal(1))
                    .title_bottom(vec![Span::from(" "), mode, edited_marker, Span::from(" ")]),
            )
            .render(area, buf);

        // FIXME: When drawing the input above
        buf.set_style(
            Rect::new(col.saturating_sub(state.scroll as u16), row, 1, 1)
                .offset(Offset { x: 2, y: 1 }),
            Style::default().reversed().dark_gray(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_input_states() {
        type TestCase = (&'static str, Box<dyn Fn() -> InputModalState>);

        let tests: Vec<TestCase> = vec![
            ("default", Box::new(InputModalState::default)),
            (
                "with_value",
                Box::new(|| InputModalState::new("Hello world", 0, true)),
            ),
            (
                "with_value_next_row",
                Box::new(|| InputModalState::new("Hello world", 1, true)),
            ),
            (
                "insert",
                Box::new(|| {
                    let mut state = InputModalState::new("", 0, true);
                    state.insert_char('B');
                    state.insert_char('a');
                    state.insert_char('s');
                    state.insert_char('a');
                    state.insert_char('l');
                    state.insert_char('t');
                    state
                }),
            ),
            (
                "delete",
                Box::new(|| {
                    let mut state = InputModalState::new("Basalt", 0, true);
                    state.cursor_left(2);
                    state.delete_char();
                    state.cursor_left(1);
                    state.delete_char();
                    state
                }),
            ),
            (
                "text_unicode",
                Box::new(|| InputModalState::new("café 世界 🎉", 0, true)),
            ),
            (
                "text_scrolled",
                Box::new(|| {
                    let mut state = InputModalState::new(
                        "This is a very long text that should trigger scrolling when rendered in the widget",
                        0,
                        true
                    );
                    // Move cursor to trigger scrolling
                    state.cursor_left(10);
                    state
                }),
            ),
            (
                "text_with_leading_spaces",
                Box::new(|| InputModalState::new("   indented text", 0, true)),
            ),
            (
                "text_with_multiple_spaces",
                Box::new(|| InputModalState::new("hello   world   test", 0, true)),
            ),
        ];

        let mut terminal = Terminal::new(TestBackend::new(30, 5)).unwrap();

        tests.into_iter().for_each(|(name, state_fn)| {
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    let mut state = state_fn();
                    Input.render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }
}
