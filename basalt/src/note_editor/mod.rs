pub mod ast;
mod cursor;
pub mod editor;
pub mod parser;
mod render;
mod rich_text;
mod state;
mod text_buffer;
mod text_wrap;
mod virtual_document;

use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Size,
};
pub use state::EditorState;
pub use text_buffer::TextBuffer;

use crate::{
    app::{calc_scroll_amount, ActivePane, Message as AppMessage, ScrollAmount},
    explorer,
    note_editor::editor::{EditMode, State, View},
    outline,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Save,
    SwitchPaneNext,
    SwitchPanePrevious,
    ToggleExplorer,
    ToggleOutline,
    ToggleView,
    EditView,
    ReadView,
    Exit,
    KeyEvent(KeyEvent),
    CursorUp,
    CursorLeft,
    CursorRight,
    CursorWordForward,
    CursorWordBackward,
    CursorDown,
    ScrollUp(ScrollAmount),
    ScrollDown(ScrollAmount),
    SetRow(usize),
    Delete,
}

// FIXME: Add resize message to handle resize related updates like cursor positioning
pub fn update<'a>(
    message: &Message,
    screen_size: Size,
    state: &mut State,
) -> Option<AppMessage<'a>> {
    match message {
        Message::CursorLeft => state.cursor_left(1),
        Message::CursorRight => state.cursor_right(1),
        // TODO: Commented until editor functionality works
        // Message::CursorWordForward => state.cursor_word_forward(),
        // Message::CursorWordBackward => state.cursor_word_backward(),
        // Message::SetRow(row) => state.set_row(*row),
        Message::CursorUp => {
            state.cursor_up(1);
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block(),
            )));
        }
        Message::CursorDown => {
            state.cursor_down(1);
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block(),
            )));
        }
        Message::ScrollUp(scroll_amount) => {
            state.cursor_up(calc_scroll_amount(scroll_amount, screen_size.height.into()));
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block(),
            )));
        }
        Message::ScrollDown(scroll_amount) => {
            state.cursor_down(calc_scroll_amount(scroll_amount, screen_size.height.into()));
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block(),
            )));
        }
        _ => {}
    };

    match state.view {
        View::Edit(..) => match message {
            Message::ToggleView => state.set_view(View::Edit(EditMode::Source)),
            // TODO: Commented until editor functionality works
            //         Message::ScrollUp(_) => state.cursor_up(),
            //         Message::ScrollDown(_) => state.cursor_down(),
            Message::KeyEvent(key) => {
                match key.code {
                    KeyCode::Char(c) => {
                        state.insert_char(c);
                    }
                    KeyCode::Enter => {
                        state.insert_char('\n');
                    }
                    _ => {}
                }

                return Some(AppMessage::UpdateSelectedNoteContent((
                    state.content.to_string(),
                    None,
                )));
            }
            Message::Delete => {
                state.delete_char();
            }
            Message::Exit => {
                state.exit_insert();
                state.set_view(View::Read);
                return Some(AppMessage::UpdateSelectedNoteContent((
                    state.content.to_string(),
                    Some(state.ast_nodes.clone()),
                )));
            }
            _ => {}
        },
        View::Read => match message {
            Message::ToggleView => state.set_view(View::Edit(EditMode::Source)),
            Message::EditView => state.set_view(View::Edit(EditMode::Source)),
            Message::ReadView => state.set_view(View::Read),
            Message::ToggleExplorer => {
                return Some(AppMessage::Explorer(explorer::Message::Toggle));
            }
            Message::ToggleOutline => {
                return Some(AppMessage::Outline(outline::Message::Toggle));
            }
            Message::SwitchPaneNext => {
                state.set_active(false);
                return Some(AppMessage::SetActivePane(ActivePane::Outline));
            }
            Message::SwitchPanePrevious => {
                state.set_active(false);
                return Some(AppMessage::SetActivePane(ActivePane::Explorer));
            }
            // TODO: Commented until editor functionality works
            //         Message::Save => {
            //             state.save();
            //             return Some(AppMessage::UpdateSelectedNoteContent((
            //                 state.content().to_string(),
            //                 None,
            //             )));
            //         }
            _ => {}
        },
    }

    None
}

pub fn handle_editing_event(key: &KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Up => Some(Message::CursorUp),
        KeyCode::Down => Some(Message::CursorDown),
        KeyCode::Left => Some(Message::CursorLeft),
        KeyCode::Right => Some(Message::CursorRight),
        KeyCode::Esc => Some(Message::Exit),
        KeyCode::Backspace => Some(Message::Delete),
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::ToggleView)
        }
        _ => Some(Message::KeyEvent(*key)),
    }
}
