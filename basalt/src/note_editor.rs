mod editor;
mod state;
mod text_buffer;

/// Provides Markdown parser that supports Obsidian flavor.
/// Obsidian flavor is a combination of different flavors and a few differences.
///
/// Namely `CommonMark` and `GitHub Flavored Markdown`. More info
/// [here](https://help.obsidian.md/Editing+and+formatting/Obsidian+Flavored+Markdown).
///
/// NOTE: Current iteration does not handle Obsidian flavor, unless it is covered by
/// pulldown-cmark. Part of Obsidian flavor is for example use of any character inside tasks to
/// mark them as completed `- [?] Completed`.
///
/// This crate uses [`pulldown_cmark`] to parse the markdown and enable the applicable features. This
/// crate uses own intermediate types to provide the parsed markdown nodes.
/// pub mod markdown;
pub mod markdown_parser;

pub use editor::Editor;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::Size,
};
pub use state::{EditorState, Mode};
pub use text_buffer::TextBuffer;

use crate::{
    app::{calc_scroll_amount, ActivePane, Message as AppMessage, ScrollAmount},
    explorer, outline,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Save,
    SwitchPaneNext,
    SwitchPanePrevious,
    ToggleExplorer,
    ToggleOutline,
    EditMode,
    ExitMode,
    ReadMode,
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

pub fn update<'a>(
    message: &Message,
    screen_size: Size,
    state: &mut EditorState,
) -> Option<AppMessage<'a>> {
    match message {
        Message::CursorLeft => state.cursor_left(),
        Message::CursorRight => state.cursor_right(),
        Message::CursorWordForward => state.cursor_word_forward(),
        Message::CursorWordBackward => state.cursor_word_backward(),
        Message::Delete => state.delete_char(),
        Message::SetRow(row) => state.set_row(*row),

        Message::CursorUp => {
            state.cursor_up();
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_row,
            )));
        }
        Message::CursorDown => {
            state.cursor_down();
            return Some(AppMessage::Outline(outline::Message::SelectAt(
                state.current_row,
            )));
        }
        _ => {}
    };

    match state.mode {
        Mode::Edit => match message {
            Message::ScrollUp(_) => state.cursor_up(),
            Message::ScrollDown(_) => state.cursor_down(),
            Message::KeyEvent(key) => {
                state.edit((*key).into());

                return Some(AppMessage::UpdateSelectedNoteContent((
                    state.content().to_string(),
                    None,
                )));
            }
            Message::ExitMode => {
                state.exit_insert();
                state.set_mode(Mode::Read);

                return Some(AppMessage::UpdateSelectedNoteContent((
                    state.content().to_string(),
                    Some(state.nodes().to_vec()),
                )));
            }
            _ => {}
        },
        Mode::View | Mode::Read => match message {
            Message::EditMode => state.set_mode(Mode::Edit),
            Message::ReadMode => state.set_mode(Mode::Read),
            Message::ExitMode => state.set_mode(Mode::View),
            Message::SetRow(row) => state.set_row(*row),

            Message::ScrollUp(scroll_amount) => {
                state.scroll_up(calc_scroll_amount(scroll_amount, screen_size.height.into()));
            }
            Message::ScrollDown(scroll_amount) => {
                state.scroll_down(calc_scroll_amount(scroll_amount, screen_size.height.into()));
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
                state.save();
                return Some(AppMessage::UpdateSelectedNoteContent((
                    state.content().to_string(),
                    None,
                )));
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
        KeyCode::Esc => Some(Message::ExitMode),
        KeyCode::Backspace => Some(Message::Delete),
        _ => Some(Message::KeyEvent(*key)),
    }
}
