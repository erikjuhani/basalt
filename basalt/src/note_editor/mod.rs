pub mod ast;
mod cursor;
pub mod editor;
pub mod parser;
mod render;
mod rich_text;
pub mod state;
mod text_buffer;
mod text_wrap;
mod viewport;
mod virtual_document;

use std::time::Duration;

use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Size,
};

use crate::{
    app::{calc_scroll_amount, ActivePane, Message as AppMessage, ScrollAmount},
    explorer,
    note_editor::state::{EditMode, NoteEditorState, View},
    outline, toast,
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
    JumpToBlock(usize),
    Delete,
}

// FIXME: Add resize message to handle resize related updates like cursor positioning
pub fn update<'a>(
    message: &Message,
    screen_size: Size,
    state: &mut NoteEditorState,
) -> Option<AppMessage<'a>> {
    match message {
        Message::CursorLeft => state.cursor_left(1),
        Message::CursorRight => state.cursor_right(1),
        Message::JumpToBlock(idx) => state.cursor_jump(*idx),
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
            Message::CursorWordForward => state.cursor_word_forward(),
            Message::CursorWordBackward => state.cursor_word_backward(),
            Message::ToggleView => state.set_view(View::Read),
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
            Message::Save => {
                let modified = state.modified();
                match state.save_to_file() {
                    Ok(_) if modified => {
                        return Some(AppMessage::Batch(vec![
                            AppMessage::UpdateSelectedNoteContent((
                                state.content.to_string(),
                                None,
                            )),
                            AppMessage::Toast(toast::Message::Create(toast::Toast::success(
                                "File saved",
                                Duration::from_secs(2),
                            ))),
                        ]))
                    }
                    // FIXME: Log the error.
                    // This requires a logging system to store system logs for debugging purposes
                    Err(_) => {
                        return Some(AppMessage::Toast(toast::Message::Create(
                            toast::Toast::error("Failed to save file", Duration::from_secs(2)),
                        )))
                    }
                    _ => {}
                }
            }
            _ => {}
        },
    }

    None
}

pub fn handle_editing_event(key: &KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Up => Some(Message::CursorUp),
        KeyCode::Down => Some(Message::CursorDown),
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::ALT) => {
            Some(Message::CursorWordForward)
        }
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::ALT) => {
            Some(Message::CursorWordBackward)
        }
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
