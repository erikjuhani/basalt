pub mod ast;
mod cursor;
pub mod editor;
pub mod parser;
mod render;
pub mod rich_text;
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
    ScrollToTop,
    ScrollToBottom,
    JumpToBlock(usize),
    Delete,
    InsertMode,
    /// Scroll the active table left by 1 display-width char (h key when cursor in table, D-12)
    TableScrollLeft,
    /// Scroll the active table right by 1 display-width char (l key when cursor in table, D-12)
    TableScrollRight,
    /// Scroll the active table left by ~1 column width (H key when cursor in table, D-12)
    TableScrollColumnLeft,
    /// Scroll the active table right by ~1 column width (L key when cursor in table, D-12)
    TableScrollColumnRight,
}

// FIXME: Add resize message to handle resize related updates like cursor positioning
pub fn update<'a>(
    message: Message,
    screen_size: Size,
    state: &mut NoteEditorState,
) -> Option<AppMessage<'a>> {
    let vim_mode = state.vim_mode();

    // Table horizontal scroll override (D-16): when cursor is in a table in Read mode,
    // h/l keys scroll the table rather than moving the cursor.
    if matches!(state.view, View::Read) && state.cursor_in_table_node().is_some() {
        match message {
            Message::CursorLeft => {
                state.table_h_scroll = state.table_h_scroll.saturating_sub(1);
                return None;
            }
            Message::CursorRight => {
                state.table_h_scroll = state.table_h_scroll.saturating_add(1);
                return None;
            }
            _ => {}
        }
    }

    match message {
        Message::CursorLeft => state.cursor_left(1),
        Message::CursorRight => state.cursor_right(1),
        Message::JumpToBlock(idx) => state.cursor_jump(idx),
        Message::CursorUp => {
            state.cursor_up(1);
            // Reset table horizontal scroll if cursor left the table (D-13)
            if state.cursor_in_table_node().is_none() {
                state.table_h_scroll = 0;
            }
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block(),
            )));
        }
        Message::CursorDown => {
            state.cursor_down(1);
            // Reset table horizontal scroll if cursor left the table (D-13)
            if state.cursor_in_table_node().is_none() {
                state.table_h_scroll = 0;
            }
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block(),
            )));
        }
        Message::ScrollUp(scroll_amount) => {
            state.cursor_up(calc_scroll_amount(
                &scroll_amount,
                screen_size.height.into(),
            ));
            // Reset table horizontal scroll if cursor left the table (D-13)
            if state.cursor_in_table_node().is_none() {
                state.table_h_scroll = 0;
            }
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block(),
            )));
        }
        Message::ScrollDown(scroll_amount) => {
            state.cursor_down(calc_scroll_amount(
                &scroll_amount,
                screen_size.height.into(),
            ));
            // Reset table horizontal scroll if cursor left the table (D-13)
            if state.cursor_in_table_node().is_none() {
                state.table_h_scroll = 0;
            }
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block(),
            )));
        }
        Message::ScrollToTop => {
            state.cursor_up(usize::MAX);
            // Reset table horizontal scroll if cursor left the table (D-13)
            if state.cursor_in_table_node().is_none() {
                state.table_h_scroll = 0;
            }
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block(),
            )));
        }
        Message::ScrollToBottom => {
            state.cursor_to_end();
            // Reset table horizontal scroll if cursor left the table (D-13)
            if state.cursor_in_table_node().is_none() {
                state.table_h_scroll = 0;
            }
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block(),
            )));
        }
        Message::TableScrollLeft => {
            state.table_h_scroll = state.table_h_scroll.saturating_sub(1);
        }
        Message::TableScrollRight => {
            state.table_h_scroll = state.table_h_scroll.saturating_add(1);
        }
        Message::TableScrollColumnLeft => {
            // Step = first column width + 3 (left-padding + right-padding + right-border)
            let col_w = state
                .table_col_widths()
                .and_then(|ws| ws.into_iter().next())
                .unwrap_or(1);
            state.table_h_scroll = state.table_h_scroll.saturating_sub(col_w + 3);
        }
        Message::TableScrollColumnRight => {
            // Step = first column width + 3 (left-padding + right-padding + right-border)
            let col_w = state
                .table_col_widths()
                .and_then(|ws| ws.into_iter().next())
                .unwrap_or(1);
            state.table_h_scroll = state.table_h_scroll.saturating_add(col_w + 3);
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
            Message::InsertMode | Message::EditView => state.set_insert_mode(true),
            Message::ToggleView | Message::ReadView => {
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
