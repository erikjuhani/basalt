use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    actions::Action,
    app::{Context, ScrollAmount},
};

// This can be expanded to be context-aware if needed.
pub fn key_event_to_action(key_event: KeyEvent, _ctx: &Context) -> Option<Action> {
    // For now, a simple global mapping. Could be: `fn key_event_to_action(key_event: KeyEvent, ctx: &Context) -> Option<Action>`
    // and then match on ctx to provide context-specific keybindings.
    match key_event.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        KeyCode::Char(' ') => Some(Action::ToggleVaultSelector),
        KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::Quit)
        }
        KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::ScrollUp(ScrollAmount::HalfPage))
        }
        KeyCode::Char('d') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::ScrollDown(ScrollAmount::HalfPage))
        }
        KeyCode::PageUp => Some(Action::ScrollUp(ScrollAmount::HalfPage)),
        KeyCode::PageDown => Some(Action::ScrollDown(ScrollAmount::HalfPage)),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Prev), // Context will determine if it's scroll or item prev
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Next), // Context will determine if it's scroll or item next
        KeyCode::Char('t') | KeyCode::Tab => Some(Action::ToggleMode),
        KeyCode::Enter => Some(Action::Select),

        // KeyCode::Esc => {
        //     // Escape could close modals or unfocus elements
        //     // This logic is better handled inside the `update` function based on context
        // }
        _ => None,
    }
}
