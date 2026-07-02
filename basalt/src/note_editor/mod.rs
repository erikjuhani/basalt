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
    note_editor::state::{EditMode, NoteEditorState, SelectionMode, View},
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
    ScrollToTop,
    ScrollToBottom,
    JumpToBlock(usize),
    Delete,
    InsertMode,
    VisualMode,
    VisualLineMode,
    Yank,
}

// FIXME: Add resize message to handle resize related updates like cursor positioning
pub fn update<'a>(
    message: Message,
    screen_size: Size,
    state: &mut NoteEditorState,
) -> Option<AppMessage<'a>> {
    let vim_mode = state.vim_mode();
    match message {
        Message::CursorLeft => state.cursor_left(1),
        Message::CursorRight => state.cursor_right(1),
        Message::JumpToBlock(idx) => state.cursor_jump(idx),
        Message::CursorUp => {
            state.cursor_up(1);
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block_idx(),
            )));
        }
        Message::CursorDown => {
            state.cursor_down(1);
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block_idx(),
            )));
        }
        Message::ScrollUp(scroll_amount) => {
            state.cursor_up(calc_scroll_amount(
                &scroll_amount,
                screen_size.height.into(),
            ));
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block_idx(),
            )));
        }
        Message::ScrollDown(scroll_amount) => {
            state.cursor_down(calc_scroll_amount(
                &scroll_amount,
                screen_size.height.into(),
            ));
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block_idx(),
            )));
        }
        Message::ScrollToTop => {
            state.cursor_up(usize::MAX);
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block_idx(),
            )));
        }
        Message::ScrollToBottom => {
            state.cursor_to_end();
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block_idx(),
            )));
        }
        _ => {}
    };

    match state.view {
        View::Edit(..) if state.insert_mode() => match message {
            Message::CursorWordForward => state.cursor_word_forward(),
            Message::CursorWordBackward => state.cursor_word_backward(),
            Message::ToggleView | Message::ReadView => {
                state.set_insert_mode(false);
                state.exit_insert();
                state.set_view(View::Read);
                return Some(AppMessage::UpdateSelectedNoteContent((
                    state.content.to_string(),
                    Some(state.ast_nodes.clone()),
                )));
            }
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
                state.set_insert_mode(false);
                state.exit_insert();
                if !vim_mode {
                    state.set_view(View::Read);
                }
                return Some(AppMessage::UpdateSelectedNoteContent((
                    state.content.to_string(),
                    Some(state.ast_nodes.clone()),
                )));
            }
            _ => {}
        },
        View::Edit(..) => match message {
            // Normal mode (vim): navigation and read-mode equivalents
            Message::CursorWordForward => state.cursor_word_forward(),
            Message::CursorWordBackward => state.cursor_word_backward(),
            Message::VisualMode => state.toggle_selection(SelectionMode::Char),
            Message::VisualLineMode => state.toggle_selection(SelectionMode::Line),
            Message::Yank => {
                if let Some(text) = state.selected_text() {
                    if let Some(range) = state.selection_range() {
                        state.flash_yank(range);
                    }
                    state.clear_selection();
                    return Some(AppMessage::CopyToClipboard(text));
                }
            }
            Message::Exit => state.clear_selection(),
            Message::InsertMode | Message::EditView => {
                state.clear_selection();
                state.set_insert_mode(true);
            }
            Message::ToggleView | Message::ReadView => {
                state.clear_selection();
                state.exit_insert();
                state.set_view(View::Read);
                return Some(AppMessage::UpdateSelectedNoteContent((
                    state.content.to_string(),
                    Some(state.ast_nodes.clone()),
                )));
            }
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
        View::Read => match message {
            Message::ToggleView if state.editor_enabled() => {
                state.set_view(View::Edit(EditMode::Source))
            }
            Message::EditView | Message::InsertMode if state.editor_enabled() => {
                state.set_view(View::Edit(EditMode::Source));
                state.set_insert_mode(true);
            }
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

pub fn handle_editing_event(key: KeyEvent) -> Option<Message> {
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
        _ => Some(Message::KeyEvent(key)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ratatui::layout::Size;

    use super::*;
    use crate::{config::Symbols, note_editor::state::EditMode};

    fn vim_edit_state(content: &str) -> NoteEditorState<'static> {
        let mut state =
            NoteEditorState::new(content, "test", Path::new("test.md"), &Symbols::unicode());
        state.set_vim_mode(true);
        state.resize_viewport(Size::new(40, 10));
        state.set_view(View::Edit(EditMode::Source));
        state
    }

    #[test]
    fn test_yank_emits_copy_to_clipboard() {
        let mut state = vim_edit_state("hello world\n");
        let size = Size::new(40, 10);

        update(Message::VisualMode, size, &mut state);
        update(Message::CursorRight, size, &mut state);
        update(Message::CursorRight, size, &mut state);
        update(Message::CursorRight, size, &mut state);
        update(Message::CursorRight, size, &mut state);

        let message = update(Message::Yank, size, &mut state);

        assert_eq!(
            message,
            Some(AppMessage::CopyToClipboard("hello".to_string()))
        );
        assert!(!state.is_selecting(), "yank should clear the selection");
        assert_eq!(
            state.yank_flash_range(),
            Some(0..5),
            "yank should flash the copied range"
        );
    }

    #[test]
    fn test_yank_without_selection_is_noop() {
        let mut state = vim_edit_state("hello world\n");
        assert_eq!(update(Message::Yank, Size::new(40, 10), &mut state), None);
    }
}
