mod search_modal;
mod search_state;

pub use search_modal::SearchModal;
pub use search_state::SearchState;

use basalt_core::obsidian::Vault;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{ActivePane, Message as AppMessage, SelectedNote};

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Toggle,
    Close,
    Up,
    Down,
    Select,
    InsertChar(char),
    DeleteChar,
    DeleteCharForward,
    DeleteWord,
    DeleteWordForward,
    DeleteToLineStart,
    DeleteToLineEnd,
    CursorLeft,
    CursorRight,
    CursorLineStart,
    CursorLineEnd,
    CursorWordForward,
    CursorWordBackward,
    Poll,
}

pub fn update<'a>(
    message: Message,
    state: &mut SearchState,
    vault: &Vault,
) -> Option<AppMessage<'a>> {
    match message {
        Message::Toggle => state.toggle(vault),
        Message::Close => state.hide(),
        Message::Up => state.move_up(),
        Message::Down => state.move_down(),
        Message::InsertChar(character) => state.insert_char(character),
        Message::DeleteChar => state.delete_char(),
        Message::DeleteCharForward => state.delete_char_forward(),
        Message::DeleteWord => state.delete_word(),
        Message::DeleteWordForward => state.delete_word_forward(),
        Message::DeleteToLineStart => state.delete_to_line_start(),
        Message::DeleteToLineEnd => state.delete_to_line_end(),
        Message::CursorLeft => state.cursor_left(),
        Message::CursorRight => state.cursor_right(),
        Message::CursorLineStart => state.cursor_line_start(),
        Message::CursorLineEnd => state.cursor_line_end(),
        Message::CursorWordForward => state.cursor_word_forward(),
        Message::CursorWordBackward => state.cursor_word_backward(),
        Message::Poll => state.poll(),
        Message::Select => {
            let target = state
                .selected_target()
                .map(|target| (SelectedNote::from(target.note), target.line, target.column));
            if let Some((note, line, column)) = target {
                state.hide();
                return Some(AppMessage::Batch(vec![
                    AppMessage::SelectNote(note),
                    AppMessage::SetActivePane(ActivePane::NoteEditor),
                    AppMessage::JumpToLine { line, column },
                ]));
            }
        }
    }

    None
}

pub fn handle_key_event(key: KeyEvent) -> Option<Message> {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    match key.code {
        KeyCode::Esc => Some(Message::Close),
        KeyCode::Enter => Some(Message::Select),
        KeyCode::Up => Some(Message::Up),
        KeyCode::Down => Some(Message::Down),
        KeyCode::Char('n') if control => Some(Message::Down),
        KeyCode::Char('p') if control => Some(Message::Up),

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
        KeyCode::Backspace => Some(Message::DeleteChar),
        KeyCode::Delete => Some(Message::DeleteCharForward),
        KeyCode::Char('d') if control => Some(Message::DeleteCharForward),
        KeyCode::Char('w') if control => Some(Message::DeleteWord),
        KeyCode::Char('d') if alt => Some(Message::DeleteWordForward),
        KeyCode::Char('u') if control => Some(Message::DeleteToLineStart),
        KeyCode::Char('k') if control => Some(Message::DeleteToLineEnd),

        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(Message::InsertChar(character))
        }
        _ => None,
    }
}
