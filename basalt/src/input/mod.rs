use basalt_core::obsidian::{directory::Directory, rename_dir, rename_note, Note};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Offset, Position, Rect},
    style::{Style, Stylize},
    text::Span,
    widgets::{Block, BorderType, Clear, Padding, Paragraph, StatefulWidget, Widget},
};

use crate::app::{ActivePane, Message as AppMessage};
use crate::config::Theme;
use crate::motion;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct TextInput {
    value: String,
    cursor: usize,
}

impl TextInput {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
            cursor: value.chars().count(),
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn set_value(&mut self, value: &str) {
        self.value = value.to_string();
        self.cursor = value.chars().count();
    }

    pub fn insert_char(&mut self, character: char) {
        self.value.insert(self.byte_index(), character);
        self.cursor_right(1);
    }

    pub fn delete_char(&mut self) {
        if self.cursor == 0 {
            return;
        }
        if let Some((byte, _)) = self.value.char_indices().nth(self.cursor - 1) {
            self.value.remove(byte);
            self.cursor_left(1);
        }
    }

    pub fn delete_char_forward(&mut self) {
        let byte = self.byte_index();
        if byte < self.value.len() {
            self.value.remove(byte);
        }
    }

    pub fn delete_word(&mut self) {
        let end = self.byte_index();
        let start = motion::word_backward(&self.value, end, false);
        self.value.replace_range(start..end, "");
        self.cursor = self.char_index(start);
    }

    pub fn delete_word_forward(&mut self) {
        let start = self.byte_index();
        let end = motion::word_forward(&self.value, start, false);
        self.value.replace_range(start..end, "");
    }

    pub fn delete_to_line_start(&mut self) {
        self.value.replace_range(..self.byte_index(), "");
        self.cursor = 0;
    }

    pub fn delete_to_line_end(&mut self) {
        self.value.truncate(self.byte_index());
    }

    pub fn cursor_left(&mut self, amount: usize) {
        self.cursor = self.clamp(self.cursor.saturating_sub(amount));
    }

    pub fn cursor_right(&mut self, amount: usize) {
        self.cursor = self.clamp(self.cursor.saturating_add(amount));
    }

    pub fn cursor_line_start(&mut self) {
        self.cursor = 0;
    }

    pub fn cursor_line_end(&mut self) {
        self.cursor = self.value.chars().count();
    }

    pub fn cursor_word_backward(&mut self) {
        self.cursor = self.char_index(motion::word_backward(&self.value, self.byte_index(), false));
    }

    pub fn cursor_word_forward(&mut self) {
        self.cursor = self.char_index(motion::word_forward(&self.value, self.byte_index(), false));
    }

    fn byte_index(&self) -> usize {
        self.value
            .char_indices()
            .map(|(byte, _)| byte)
            .nth(self.cursor)
            .unwrap_or(self.value.len())
    }

    fn char_index(&self, byte: usize) -> usize {
        self.value[..byte].chars().count()
    }

    fn clamp(&self, cursor: usize) -> usize {
        cursor.min(self.value.chars().count())
    }
}

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
    text: TextInput,
    input_original: String,
    cursor_row: usize,
    input_mode: InputMode,
    scroll: usize,
    visible: bool,
    label: String,
    offset_x: usize,
    callback: Option<Callback>,
    pub(crate) terminal_cursor: Option<Position>,
}

impl InputModalState {
    pub fn new(value: &str, row: usize, visible: bool) -> Self {
        Self {
            text: TextInput::new(value),
            input_original: value.to_string(),
            cursor_row: row,
            input_mode: InputMode::Editing,
            scroll: 0,
            offset_x: 0,
            visible,
            label: String::from("Input"),
            callback: None,
            terminal_cursor: None,
        }
    }

    pub fn set_input(&mut self, value: &str) {
        self.text.set_value(value);
        self.input_original = value.to_string();
        self.scroll = 0;
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
                    rename_note(note.clone(), self.text.value())
                        .ok()
                        .map(|n| (original_path, n.path().to_path_buf()))
                }
                Callback::RenameDir(directory) => {
                    let original_path = directory.path().to_path_buf();
                    rename_dir(directory.clone(), self.text.value())
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

    pub fn modified(&self) -> bool {
        self.text.value() != self.input_original
    }

    pub fn insert_char(&mut self, char: char) {
        self.text.insert_char(char);
    }

    pub fn delete_char(&mut self) {
        self.text.delete_char();
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
    CursorLineStart,
    CursorLineEnd,
    CursorWordForward,
    CursorWordBackward,
    Open(InputModalConfig),
    Accept,
    Delete,
    DeleteForward,
    DeleteWord,
    DeleteWordForward,
    DeleteToLineStart,
    DeleteToLineEnd,
    KeyEvent(KeyEvent),
    Cancel,
    EditMode,
}

pub fn update<'a>(message: Message, state: &mut InputModalState) -> Option<AppMessage<'a>> {
    match message {
        Message::CursorLeft => {
            state.text.cursor_left(1);
        }
        Message::CursorRight => {
            state.text.cursor_right(1);
        }
        Message::CursorLineStart => {
            state.text.cursor_line_start();
        }
        Message::CursorLineEnd => {
            state.text.cursor_line_end();
        }
        Message::CursorWordForward => {
            state.text.cursor_word_forward();
        }
        Message::CursorWordBackward => {
            state.text.cursor_word_backward();
        }
        Message::DeleteForward => state.text.delete_char_forward(),
        Message::DeleteWord => state.text.delete_word(),
        Message::DeleteWordForward => state.text.delete_word_forward(),
        Message::DeleteToLineStart => state.text.delete_to_line_start(),
        Message::DeleteToLineEnd => state.text.delete_to_line_end(),
        Message::Cancel => match state.input_mode {
            InputMode::Editing => state.input_mode = InputMode::Normal,
            InputMode::Normal => {
                state.toggle_visibility();
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
                if state.modified() {
                    let rename = state.run_callback();
                    state.input_mode = InputMode::Normal;
                    state.toggle_visibility();
                    let select = rename.as_ref().map(|(_, new)| new.clone());
                    return Some(AppMessage::RefreshVault { rename, select });
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
            state.set_input(&initial_input);
            state.set_row(position.y as usize);
            state.set_offset_x(position.x as usize);
            state.set_label(&label);
            state.set_callback(&callback);
            state.toggle_visibility();
            return Some(AppMessage::SetActivePane(ActivePane::Input));
        }
        Message::Delete => state.delete_char(),
        _ => {}
    }

    None
}

pub fn handle_editing_event(key: KeyEvent) -> Option<Message> {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    match key.code {
        KeyCode::Esc => Some(Message::Cancel),

        KeyCode::Left if control => Some(Message::CursorWordBackward),
        KeyCode::Right if control => Some(Message::CursorWordForward),
        KeyCode::Left => Some(Message::CursorLeft),
        KeyCode::Right => Some(Message::CursorRight),
        KeyCode::Home => Some(Message::CursorLineStart),
        KeyCode::End => Some(Message::CursorLineEnd),
        KeyCode::Char('a') if control => Some(Message::CursorLineStart),
        KeyCode::Char('e') if control => Some(Message::CursorLineEnd),
        KeyCode::Char('b') if alt => Some(Message::CursorWordBackward),
        KeyCode::Char('f') if alt => Some(Message::CursorWordForward),

        KeyCode::Backspace if alt => Some(Message::DeleteWord),
        KeyCode::Backspace => Some(Message::Delete),
        KeyCode::Delete => Some(Message::DeleteForward),
        KeyCode::Char('d') if control => Some(Message::DeleteForward),
        KeyCode::Char('w') if control => Some(Message::DeleteWord),
        KeyCode::Char('d') if alt => Some(Message::DeleteWordForward),
        KeyCode::Char('u') if control => Some(Message::DeleteToLineStart),
        KeyCode::Char('k') if control => Some(Message::DeleteToLineEnd),

        _ => Some(Message::KeyEvent(key)),
    }
}

#[derive(Clone, Debug, Default)]
pub struct Input {
    pub border_type: BorderType,
    pub theme: Theme,
}

impl Input {
    pub fn new(border_type: BorderType, theme: Theme) -> Self {
        Self { border_type, theme }
    }
}

impl StatefulWidget for Input {
    type State = InputModalState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.terminal_cursor = None;
        if !state.visible {
            return;
        }

        // Input widget height is set to 3 since we include the borders
        let height = 3;

        let width = 40u16
            .saturating_sub((state.offset_x * 2) as u16)
            .min(area.width);

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

        let row = area.top();
        let cursor = state.text.cursor();
        let col = cursor as u16 + area.left();

        // The block borders and horizontal padding take two columns on each
        // side, so the text fits in `area.width - 4` columns.
        let text_width = area.width.saturating_sub(4) as usize;

        if cursor >= state.scroll + text_width {
            state.scroll = cursor + 1 - text_width;
        } else if cursor < state.scroll {
            state.scroll = cursor;
        }

        // A deletion or a shrunk area can leave the scroll offset past the text,
        // leaving empty space at the right. Pull the window left so the text
        // stays filled, and never past the last character.
        let value_len = state.text.value().chars().count();
        state.scroll = state
            .scroll
            .min((value_len + 1).saturating_sub(text_width))
            .min(value_len);

        let start = state
            .text
            .value()
            .char_indices()
            .nth(state.scroll)
            .map_or(state.text.value().len(), |(byte, _)| byte);
        let input = &state.text.value()[start..];

        let mode_color = match state.input_mode {
            InputMode::Editing => self.theme.success,
            InputMode::Normal => self.theme.error,
        };

        let mode = format!("{:?}", state.input_mode)
            .fg(mode_color)
            .bold()
            .italic();

        let edited_marker = if state.modified() {
            "*".bold().italic()
        } else {
            "".into()
        };

        Paragraph::new(input)
            .block(
                Block::bordered()
                    .border_type(self.border_type)
                    .border_style(Style::default().fg(self.theme.muted))
                    .style(Style::default().bg(self.theme.background))
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

        let cursor = Rect::new(col.saturating_sub(state.scroll as u16), row, 1, 1)
            .offset(Offset { x: 2, y: 1 });
        state.terminal_cursor = Some(Position::new(cursor.x, cursor.y));
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
                    state.text.cursor_left(2);
                    state.delete_char();
                    state.text.cursor_left(1);
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
                    state.text.cursor_left(10);
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
                    Input::new(BorderType::Rounded, Theme::default()).render(
                        frame.area(),
                        frame.buffer_mut(),
                        &mut state,
                    )
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }

    /// A `TextInput` holding `value` with the cursor moved `left` chars back
    /// from the end.
    fn at(value: &str, left: usize) -> TextInput {
        let mut text = TextInput::new(value);
        text.cursor_left(left);
        text
    }

    #[test]
    fn cursor_line_motions_jump_to_the_ends() {
        let mut text = at("hello world", 3);
        text.cursor_line_start();
        assert_eq!(text.cursor(), 0);
        text.cursor_line_end();
        assert_eq!(text.cursor(), 11);
    }

    #[test]
    fn word_motions_stop_at_vim_boundaries() {
        let mut text = TextInput::new("foo-bar baz");
        text.cursor_word_backward();
        assert_eq!(text.cursor(), 8);
        text.cursor_word_backward();
        assert_eq!(text.cursor(), 4);
    }

    #[test]
    fn delete_char_forward_removes_the_char_under_the_cursor() {
        let mut text = at("abc", 3);
        text.delete_char_forward();
        assert_eq!(text.value(), "bc");
        assert_eq!(text.cursor(), 0);
    }

    #[test]
    fn delete_word_forward_removes_to_the_next_word() {
        let mut text = at("foo bar", 7);
        text.delete_word_forward();
        assert_eq!(text.value(), "bar");
        assert_eq!(text.cursor(), 0);
    }

    #[test]
    fn delete_to_line_start_and_end_cut_around_the_cursor() {
        let mut text = at("hello world", 5);
        text.delete_to_line_start();
        assert_eq!(text.value(), "world");
        assert_eq!(text.cursor(), 0);

        let mut text = at("hello world", 6);
        text.delete_to_line_end();
        assert_eq!(text.value(), "hello");
    }

    #[test]
    fn deletes_respect_char_boundaries() {
        let mut text = at("café 世界", 2);
        text.delete_word_forward();
        assert_eq!(text.value(), "café ");
    }
}
