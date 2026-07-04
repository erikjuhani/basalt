pub mod ast;
mod cursor;
pub mod editor;
mod motion;
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
    note_editor::state::{EditMode, FindKind, NoteEditorState, Operator, SelectionMode, View},
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
    CursorLineStart,
    CursorLineEnd,
    CursorFirstNonblank,
    CursorWordEnd,
    CursorWordForwardBig,
    CursorWordBackwardBig,
    CursorWordEndBig,
    ParagraphForward,
    ParagraphBackward,
    MatchingPair,
    CursorDocStart,
    CursorDocEnd,
    FindChar { forward: bool, till: bool },
    RepeatFind { reverse: bool },
    CountDigit(u8),
    FindTarget(char),
    TextObjectTarget(char),
    ReplaceChar,
    ReplaceTarget(char),
    Append,
    Operator(Operator),
    DeleteUnderCursor,
    DeleteToLineEnd,
    ChangeToLineEnd,
    SubstituteChar,
    PasteAfter,
    PasteBefore,
    Undo,
    Redo,
}

fn offset(state: &NoteEditorState) -> usize {
    state.cursor.source_offset()
}

fn content_update<'a>(state: &NoteEditorState) -> AppMessage<'a> {
    AppMessage::UpdateSelectedNoteContent((
        state.content.to_string(),
        Some(state.ast_nodes.clone()),
    ))
}

fn run_motion<'a>(
    state: &mut NoteEditorState,
    count: usize,
    inclusive: bool,
    linewise: bool,
    motion: impl Fn(&str, usize) -> usize,
) -> Option<AppMessage<'a>> {
    let target = (0..count).fold(offset(state), |from, _| motion(&state.content, from));
    match state.take_operator() {
        Some(operator) => operate(state, operator, target, inclusive, linewise),
        None => motion_to(state, target),
    }
}

fn apply_find<'a>(
    state: &mut NoteEditorState,
    target: char,
    kind: FindKind,
    count: usize,
) -> Option<AppMessage<'a>> {
    run_motion(state, count, kind.forward, false, move |content, from| {
        motion::find_char(content, from, target, kind.forward, kind.till).unwrap_or(from)
    })
}

fn operate<'a>(
    state: &mut NoteEditorState,
    operator: Operator,
    target: usize,
    inclusive: bool,
    linewise: bool,
) -> Option<AppMessage<'a>> {
    let cursor = offset(state);
    let (lo, hi) = (cursor.min(target), cursor.max(target));
    let range = if linewise {
        let start = state.content[..lo].rfind('\n').map_or(0, |i| i + 1);
        let end = state.content[hi..]
            .find('\n')
            .map_or(state.content.len(), |i| hi + i + 1);
        start..end
    } else {
        let end = if inclusive {
            hi + state.content[hi..].chars().next().map_or(0, char::len_utf8)
        } else {
            hi
        };
        lo..end
    };
    match operator {
        Operator::Yank => yank(state, range, linewise),
        Operator::Delete => delete(state, range, linewise),
        Operator::Change => change(state, range, linewise),
    }
}

fn yank<'a>(
    state: &mut NoteEditorState,
    range: core::ops::Range<usize>,
    linewise: bool,
) -> Option<AppMessage<'a>> {
    let text = state.content.get(range.clone())?.to_string();
    if text.is_empty() {
        return None;
    }
    state.set_register(text.clone(), linewise);
    state.flash_yank(range.clone());
    state.jump_to_offset(range.start);
    Some(AppMessage::CopyToClipboard(text))
}

fn delete<'a>(
    state: &mut NoteEditorState,
    range: core::ops::Range<usize>,
    linewise: bool,
) -> Option<AppMessage<'a>> {
    let text = state.content.get(range.clone())?.to_string();
    if text.is_empty() {
        return None;
    }
    state.set_register(text, linewise);
    state.splice(range, "");
    Some(content_update(state))
}

fn change<'a>(
    state: &mut NoteEditorState,
    range: core::ops::Range<usize>,
    linewise: bool,
) -> Option<AppMessage<'a>> {
    let text = state.content.get(range.clone())?.to_string();
    state.set_register(text, linewise);
    state.splice(range, "");
    state.set_insert_mode(true);
    Some(content_update(state))
}

fn apply_to_selection<'a>(
    state: &mut NoteEditorState,
    operator: Operator,
) -> Option<AppMessage<'a>> {
    let linewise = matches!(
        state.selection().map(|selection| selection.mode),
        Some(SelectionMode::Line)
    );
    let range = state.selection_range()?;
    state.clear_selection();
    match operator {
        Operator::Yank => yank(state, range, linewise),
        Operator::Delete => delete(state, range, linewise),
        Operator::Change => change(state, range, linewise),
    }
}

fn operate_lines<'a>(
    state: &mut NoteEditorState,
    operator: Operator,
    count: usize,
) -> Option<AppMessage<'a>> {
    let cursor = offset(state);
    let start = state.content[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let end = (0..count).fold(start, |line, _| {
        state.content[line..]
            .find('\n')
            .map_or(state.content.len(), |i| line + i + 1)
    });
    match operator {
        Operator::Yank => yank(state, start..end, true),
        Operator::Delete => delete(state, start..end, true),
        Operator::Change => change(state, start..end, true),
    }
}

fn motion_to<'a>(state: &mut NoteEditorState, offset: usize) -> Option<AppMessage<'a>> {
    state.jump_to_offset(offset);
    Some(AppMessage::Outline(outline::Message::SelectAt(
        state.current_block_idx(),
    )))
}

// FIXME: Add resize message to handle resize related updates like cursor positioning
pub fn update<'a>(
    message: Message,
    screen_size: Size,
    state: &mut NoteEditorState,
) -> Option<AppMessage<'a>> {
    let vim_mode = state.vim_mode();

    if !matches!(message, Message::FindTarget(_)) {
        state.clear_pending_find();
    }
    if !matches!(message, Message::TextObjectTarget(_)) {
        state.clear_pending_text_object();
    }
    if !matches!(message, Message::ReplaceTarget(_)) {
        state.clear_pending_replace();
    }

    match message {
        Message::CursorLeft => {
            let count = state.take_count().unwrap_or(1);
            if state.pending_operator().is_some() {
                return run_motion(state, count, false, false, |content, from| {
                    motion::nth_char_left(content, from, 1)
                });
            }
            state.cursor_left(count);
        }
        Message::CursorRight => {
            let count = state.take_count().unwrap_or(1);
            if state.pending_operator().is_some() {
                return run_motion(state, count, false, false, |content, from| {
                    motion::nth_char_right(content, from, 1)
                });
            }
            state.cursor_right(count);
        }
        Message::JumpToBlock(idx) => state.cursor_jump(idx),
        Message::CursorUp => {
            let count = state.take_count().unwrap_or(1);
            if state.pending_operator().is_some() {
                return run_motion(state, 1, false, true, move |content, from| {
                    motion::line_up(content, from, count)
                });
            }
            state.cursor_up(count);
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_block_idx(),
            )));
        }
        Message::CursorDown => {
            let count = state.take_count().unwrap_or(1);
            if state.pending_operator().is_some() {
                return run_motion(state, 1, false, true, move |content, from| {
                    motion::line_down(content, from, count)
                });
            }
            state.cursor_down(count);
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
            Message::CursorWordForward => {
                let target = motion::word_forward(&state.content, offset(state), false);
                state.jump_to_offset(target);
            }
            Message::CursorWordBackward => {
                let target = motion::word_backward(&state.content, offset(state), false);
                state.jump_to_offset(target);
            }
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
            Message::CursorWordForward => {
                let count = state.take_count().unwrap_or(1);
                // vim quirk: `cw` acts like `ce`, changing to the word end
                // instead of swallowing the trailing whitespace.
                if state.pending_operator() == Some(Operator::Change) {
                    return run_motion(state, count, true, false, |content, from| {
                        motion::word_end(content, from, false)
                    });
                }
                return run_motion(state, count, false, false, |content, from| {
                    motion::word_forward(content, from, false)
                });
            }
            Message::CursorWordBackward => {
                let count = state.take_count().unwrap_or(1);
                return run_motion(state, count, false, false, |content, from| {
                    motion::word_backward(content, from, false)
                });
            }
            Message::CursorWordEnd => {
                let count = state.take_count().unwrap_or(1);
                return run_motion(state, count, true, false, |content, from| {
                    motion::word_end(content, from, false)
                });
            }
            Message::CursorWordForwardBig => {
                let count = state.take_count().unwrap_or(1);
                if state.pending_operator() == Some(Operator::Change) {
                    return run_motion(state, count, true, false, |content, from| {
                        motion::word_end(content, from, true)
                    });
                }
                return run_motion(state, count, false, false, |content, from| {
                    motion::word_forward(content, from, true)
                });
            }
            Message::CursorWordBackwardBig => {
                let count = state.take_count().unwrap_or(1);
                return run_motion(state, count, false, false, |content, from| {
                    motion::word_backward(content, from, true)
                });
            }
            Message::CursorWordEndBig => {
                let count = state.take_count().unwrap_or(1);
                return run_motion(state, count, true, false, |content, from| {
                    motion::word_end(content, from, true)
                });
            }
            Message::CursorLineStart => {
                state.reset_count();
                return run_motion(state, 1, false, false, motion::line_start);
            }
            Message::CursorLineEnd => {
                state.reset_count();
                return run_motion(state, 1, true, false, motion::line_end);
            }
            Message::CursorFirstNonblank => {
                state.reset_count();
                return run_motion(state, 1, false, false, motion::first_nonblank);
            }
            Message::ParagraphForward => {
                let count = state.take_count().unwrap_or(1);
                return run_motion(state, count, false, false, motion::paragraph_forward);
            }
            Message::ParagraphBackward => {
                let count = state.take_count().unwrap_or(1);
                return run_motion(state, count, false, false, motion::paragraph_backward);
            }
            Message::MatchingPair => {
                state.reset_count();
                let target = motion::matching_pair(&state.content, offset(state));
                return match (state.take_operator(), target) {
                    (Some(operator), Some(target)) => operate(state, operator, target, true, false),
                    (None, Some(target)) => motion_to(state, target),
                    _ => None,
                };
            }
            Message::CursorDocStart => {
                let count = state.take_count();
                return run_motion(state, 1, false, true, move |content, _| {
                    count.map_or_else(
                        || motion::doc_start(content),
                        |line| motion::goto_line(content, line),
                    )
                });
            }
            Message::CursorDocEnd => {
                let count = state.take_count();
                return run_motion(state, 1, false, true, move |content, _| {
                    count.map_or_else(
                        || motion::doc_end(content),
                        |line| motion::goto_line(content, line),
                    )
                });
            }
            Message::Operator(operator) => {
                if state.is_selecting() {
                    return apply_to_selection(state, operator);
                }
                match state.pending_operator() {
                    Some(pending) if pending == operator => {
                        let count = state.take_count().unwrap_or(1);
                        state.clear_operator();
                        return operate_lines(state, operator, count);
                    }
                    _ => state.set_operator(operator),
                }
            }
            Message::DeleteUnderCursor => {
                let count = state.take_count().unwrap_or(1);
                state.clear_operator();
                let cursor = offset(state);
                let end = motion::nth_char_right(&state.content, cursor, count);
                return delete(state, cursor..end, false);
            }
            Message::DeleteToLineEnd => {
                state.clear_operator();
                let cursor = offset(state);
                let end = motion::line_end_exclusive(&state.content, cursor);
                return delete(state, cursor..end, false);
            }
            Message::ChangeToLineEnd => {
                state.clear_operator();
                let cursor = offset(state);
                let end = motion::line_end_exclusive(&state.content, cursor);
                return change(state, cursor..end, false);
            }
            Message::SubstituteChar => {
                let count = state.take_count().unwrap_or(1);
                state.clear_operator();
                let cursor = offset(state);
                let end = motion::nth_char_right(&state.content, cursor, count);
                return change(state, cursor..end, false);
            }
            Message::PasteAfter => {
                state.clear_operator();
                state.paste(true);
                return Some(content_update(state));
            }
            Message::PasteBefore => {
                state.clear_operator();
                state.paste(false);
                return Some(content_update(state));
            }
            Message::Undo => {
                state.clear_operator();
                if state.undo() {
                    return Some(content_update(state));
                }
            }
            Message::Redo => {
                state.clear_operator();
                if state.redo() {
                    return Some(content_update(state));
                }
            }
            Message::FindChar { forward, till } => state.arm_find(forward, till),
            Message::FindTarget(character) => {
                let count = state.take_count().unwrap_or(1);
                if let Some(kind) = state.take_pending_find() {
                    state.remember_find(character, kind);
                    return apply_find(state, character, kind, count);
                }
            }
            Message::RepeatFind { reverse } => {
                let count = state.take_count().unwrap_or(1);
                if let Some((target, kind)) = state.last_find() {
                    let kind = if reverse {
                        FindKind {
                            forward: !kind.forward,
                            till: kind.till,
                        }
                    } else {
                        kind
                    };
                    return apply_find(state, target, kind, count);
                }
            }
            Message::CountDigit(digit) => state.push_count_digit(digit),
            Message::VisualMode => {
                state.clear_operator();
                state.toggle_selection(SelectionMode::Char);
            }
            Message::VisualLineMode => {
                state.clear_operator();
                state.toggle_selection(SelectionMode::Line);
            }
            Message::Exit => {
                state.reset_count();
                state.clear_operator();
                state.clear_selection();
            }
            Message::InsertMode if state.pending_operator().is_some() => {
                state.arm_text_object(false);
            }
            Message::InsertMode | Message::EditView => {
                state.reset_count();
                state.clear_operator();
                state.clear_selection();
                state.mark_undo_point();
                state.set_insert_mode(true);
            }
            Message::Append => {
                if state.pending_operator().is_some() {
                    state.arm_text_object(true);
                } else {
                    state.reset_count();
                    state.clear_selection();
                    state.mark_undo_point();
                    let target = motion::nth_char_right(&state.content, offset(state), 1);
                    state.jump_to_offset(target);
                    state.set_insert_mode(true);
                }
            }
            Message::TextObjectTarget(object) => {
                let around = state.take_text_object();
                let operator = state.take_operator();
                if let (Some(around), Some(operator)) = (around, operator) {
                    if let Some(range) =
                        motion::text_object(&state.content, offset(state), object, around)
                    {
                        return match operator {
                            Operator::Yank => yank(state, range, false),
                            Operator::Delete => delete(state, range, false),
                            Operator::Change => change(state, range, false),
                        };
                    }
                }
            }
            Message::ReplaceChar => {
                state.clear_operator();
                state.arm_replace();
            }
            Message::ReplaceTarget(character) => {
                state.clear_pending_replace();
                let count = state.take_count().unwrap_or(1);
                let cursor = offset(state);
                let end = motion::nth_char_right(&state.content, cursor, count);
                let replaced = state.content[cursor..end].chars().count();
                if replaced > 0 {
                    let replacement = character.to_string().repeat(replaced);
                    let landing = (cursor + replacement.len()).saturating_sub(character.len_utf8());
                    state.splice(cursor..end, &replacement);
                    state.jump_to_offset(landing);
                    return Some(content_update(state));
                }
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

        let message = update(Message::Operator(Operator::Yank), size, &mut state);

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
    fn test_yank_without_selection_arms_operator() {
        let mut state = vim_edit_state("hello world\n");
        // y with no selection arms the yank operator rather than copying.
        assert_eq!(
            update(
                Message::Operator(Operator::Yank),
                Size::new(40, 10),
                &mut state
            ),
            None
        );
        assert_eq!(state.pending_operator(), Some(Operator::Yank));
    }

    #[test]
    fn test_line_and_word_motions() {
        let mut state = vim_edit_state("foo bar baz\n");
        let size = Size::new(40, 10);

        update(Message::CursorLineEnd, size, &mut state);
        assert_eq!(state.cursor.source_offset(), 10, "$ lands on the last char");

        update(Message::CursorLineStart, size, &mut state);
        assert_eq!(state.cursor.source_offset(), 0, "0 lands on the first col");

        update(Message::CursorWordForward, size, &mut state);
        assert_eq!(state.cursor.source_offset(), 4, "w lands on 'bar'");

        update(Message::CursorWordEnd, size, &mut state);
        assert_eq!(state.cursor.source_offset(), 6, "e lands on end of 'bar'");

        update(Message::CursorWordBackward, size, &mut state);
        assert_eq!(state.cursor.source_offset(), 4, "b lands on start of 'bar'");
    }

    #[test]
    fn insert_mode_word_motion_uses_engine() {
        // In insert mode (alt+f / alt+b) word motion now shares the vim engine,
        // so it stops at punctuation instead of splitting only on spaces.
        let mut state = vim_edit_state("foo.bar baz\n");
        state.set_insert_mode(true);
        let size = Size::new(40, 10);

        update(Message::CursorWordForward, size, &mut state);
        assert_eq!(
            state.cursor.source_offset(),
            3,
            "alt+f stops at the '.' punctuation boundary"
        );

        update(Message::CursorWordBackward, size, &mut state);
        assert_eq!(
            state.cursor.source_offset(),
            0,
            "alt+b returns to the word start"
        );
    }

    #[test]
    fn test_motion_crosses_block_boundary() {
        let mut state = vim_edit_state("# Title\n\nsecond paragraph\n");
        let size = Size::new(40, 12);

        // Jump straight to the last block, then to its end.
        update(Message::CursorDocEnd, size, &mut state);
        let content = state.content.clone();
        let second = content.find("second").unwrap();
        assert!(
            state.cursor.source_offset() >= second,
            "G reaches the second block (offset {} >= {second})",
            state.cursor.source_offset(),
        );
        assert!(state.current_block_idx() >= 1, "cursor is in a later block");

        update(Message::CursorDocStart, size, &mut state);
        assert_eq!(
            state.cursor.source_offset(),
            0,
            "gg returns to the first block"
        );
        assert_eq!(state.current_block_idx(), 0);
    }

    #[test]
    fn test_find_char_and_repeat() {
        // a b c x d e f x
        let mut state = vim_edit_state("abcxdefx\n");
        let size = Size::new(40, 10);

        update(
            Message::FindChar {
                forward: true,
                till: false,
            },
            size,
            &mut state,
        );
        update(Message::FindTarget('x'), size, &mut state);
        assert_eq!(state.cursor.source_offset(), 3, "f x -> first x");

        update(Message::RepeatFind { reverse: false }, size, &mut state);
        assert_eq!(state.cursor.source_offset(), 7, "; -> next x");

        update(Message::RepeatFind { reverse: true }, size, &mut state);
        assert_eq!(state.cursor.source_offset(), 3, ", -> back to first x");
    }

    #[test]
    fn test_till_stops_before_target() {
        let mut state = vim_edit_state("abcxdef\n");
        let size = Size::new(40, 10);
        update(
            Message::FindChar {
                forward: true,
                till: true,
            },
            size,
            &mut state,
        );
        update(Message::FindTarget('x'), size, &mut state);
        assert_eq!(state.cursor.source_offset(), 2, "t x -> just before x");
    }

    #[test]
    fn test_count_repeats_word_motion() {
        // one two three four
        let mut state = vim_edit_state("one two three four\n");
        let size = Size::new(60, 10);

        update(Message::CountDigit(3), size, &mut state);
        update(Message::CursorWordForward, size, &mut state);
        assert_eq!(state.cursor.source_offset(), 14, "3w -> start of 'four'");
    }

    #[test]
    fn test_multi_digit_count() {
        let mut state = vim_edit_state("abcdefghijklmno\n");
        let size = Size::new(40, 10);

        update(Message::CountDigit(1), size, &mut state);
        update(Message::CountDigit(2), size, &mut state);
        update(Message::CursorRight, size, &mut state);
        assert_eq!(state.cursor.source_offset(), 12, "12l -> offset 12");
    }

    fn delete(state: &mut NoteEditorState) {
        update(
            Message::Operator(Operator::Delete),
            Size::new(60, 12),
            state,
        );
    }

    #[test]
    fn test_delete_word() {
        let mut state = vim_edit_state("foo bar baz\n");
        delete(&mut state);
        update(Message::CursorWordForward, Size::new(60, 12), &mut state);
        assert_eq!(
            state.content, "bar baz\n",
            "dw removes the first word and space"
        );
    }

    #[test]
    fn test_delete_line_doubled_operator() {
        let mut state = vim_edit_state("line one\nline two\nline three\n");
        delete(&mut state);
        delete(&mut state); // dd
        assert_eq!(state.content, "line two\nline three\n");
    }

    #[test]
    fn test_change_word_is_change_to_end() {
        let mut state = vim_edit_state("foo bar\n");
        let size = Size::new(60, 12);
        update(Message::Operator(Operator::Change), size, &mut state);
        update(Message::CursorWordForward, size, &mut state);
        assert_eq!(
            state.content, " bar\n",
            "cw changes to word end, keeping the space"
        );
        assert!(state.insert_mode(), "change enters insert mode");
    }

    #[test]
    fn test_delete_char_under_cursor() {
        let mut state = vim_edit_state("abc\n");
        update(Message::DeleteUnderCursor, Size::new(40, 10), &mut state);
        assert_eq!(state.content, "bc\n");
        assert_eq!(state.register().text, "a");
    }

    #[test]
    fn test_yank_line_and_paste() {
        let mut state = vim_edit_state("one\ntwo\n");
        let size = Size::new(40, 10);
        update(Message::Operator(Operator::Yank), size, &mut state);
        update(Message::Operator(Operator::Yank), size, &mut state); // yy
        assert!(state.register().linewise);
        update(Message::PasteAfter, size, &mut state);
        assert_eq!(state.content, "one\none\ntwo\n", "p pastes the line below");
    }

    #[test]
    fn test_undo_redo() {
        let mut state = vim_edit_state("hello\n");
        let size = Size::new(40, 10);
        delete(&mut state);
        delete(&mut state); // dd -> empty
        assert_eq!(state.content, "");
        update(Message::Undo, size, &mut state);
        assert_eq!(state.content, "hello\n", "u restores the deleted line");
        update(Message::Redo, size, &mut state);
        assert_eq!(state.content, "", "ctrl+r reapplies the delete");
    }

    #[test]
    fn test_change_inner_quotes() {
        let mut state = vim_edit_state("say \"hello\" now\n");
        let size = Size::new(40, 10);
        update(Message::Operator(Operator::Change), size, &mut state); // c
        update(Message::InsertMode, size, &mut state); // i (inner, operator pending)
        update(Message::TextObjectTarget('"'), size, &mut state); // "
        assert_eq!(
            state.content, "say \"\" now\n",
            "ci\" clears inside the quotes"
        );
        assert!(state.insert_mode(), "change enters insert mode");
    }

    #[test]
    fn test_change_word_in_second_block() {
        let mut state = vim_edit_state("# Title\n\nThe quick brown fox\n");
        let size = Size::new(50, 12);
        let t = state.content.find("The").unwrap();
        state.jump_to_offset(t);
        update(Message::Operator(Operator::Change), size, &mut state);
        update(Message::CursorWordForward, size, &mut state);
        for character in "swift".chars() {
            update(
                Message::KeyEvent(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)),
                size,
                &mut state,
            );
        }
        update(Message::Exit, size, &mut state);
        assert_eq!(state.content, "# Title\n\nswift quick brown fox\n");
    }

    #[test]
    fn test_replace_char() {
        let mut state = vim_edit_state("cat\n");
        let size = Size::new(40, 10);
        update(Message::ReplaceChar, size, &mut state);
        update(Message::ReplaceTarget('b'), size, &mut state);
        assert_eq!(state.content, "bat\n");
        assert!(!state.insert_mode(), "r stays in normal mode");
        assert_eq!(
            state.cursor.source_offset(),
            0,
            "cursor stays on replaced char"
        );
        assert!(
            !state.awaiting_replace(),
            "replace is one-shot: the next key is not swallowed"
        );
    }

    #[test]
    fn test_replace_char_with_count() {
        let mut state = vim_edit_state("cat\n");
        let size = Size::new(40, 10);
        update(Message::CountDigit(3), size, &mut state);
        update(Message::ReplaceChar, size, &mut state);
        update(Message::ReplaceTarget('x'), size, &mut state);
        assert_eq!(state.content, "xxx\n", "3rx replaces three chars");
    }

    #[test]
    fn test_delete_around_parens() {
        let mut state = vim_edit_state("call(a, b)\n");
        let size = Size::new(40, 10);
        // Move onto the '(' first — bracket objects need the cursor inside the pair.
        update(
            Message::FindChar {
                forward: true,
                till: false,
            },
            size,
            &mut state,
        );
        update(Message::FindTarget('('), size, &mut state);
        update(Message::Operator(Operator::Delete), size, &mut state); // d
        update(Message::Append, size, &mut state); // a (around, operator pending)
        update(Message::TextObjectTarget('('), size, &mut state);
        assert_eq!(state.content, "call\n", "da( removes the whole (...)");
    }

    #[test]
    fn test_word_forward_big_skips_punctuation() {
        let mut state = vim_edit_state("foo.bar baz\n");
        update(Message::CursorWordForwardBig, Size::new(60, 10), &mut state);
        assert_eq!(
            state.cursor.source_offset(),
            8,
            "W treats foo.bar as one WORD"
        );
    }

    #[test]
    fn test_visual_delete_selection() {
        let mut state = vim_edit_state("hello world\n");
        let size = Size::new(40, 10);
        update(Message::VisualMode, size, &mut state);
        for _ in 0..4 {
            update(Message::CursorRight, size, &mut state);
        }
        update(Message::Operator(Operator::Delete), size, &mut state);
        assert_eq!(
            state.content, " world\n",
            "v + motion + d deletes the selection"
        );
    }
}
