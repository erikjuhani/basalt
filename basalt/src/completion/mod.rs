mod dropdown;
mod state;

pub use dropdown::Dropdown;
pub use state::CompletionState;

use basalt_core::obsidian::Note;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{app::Message as AppMessage, note_editor::state::NoteEditorState};

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Up,
    Down,
    Accept,
    Dismiss,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Context<'a> {
    pub(crate) anchor: usize,
    pub(crate) query: &'a str,
}

pub(crate) fn context(line_start: usize, before_cursor: &str) -> Option<Context<'_>> {
    let open = before_cursor.rfind("[[")?;
    let query = &before_cursor[open + 2..];
    (!query.contains("]]") && !query.contains('|')).then_some(Context {
        anchor: line_start + open,
        query,
    })
}

pub fn opens_completion(message: &crate::note_editor::Message) -> bool {
    matches!(
        message,
        crate::note_editor::Message::KeyEvent(KeyEvent {
            code: KeyCode::Char('['),
            ..
        })
    )
}

pub fn sync(
    state: &mut CompletionState,
    editor: &NoteEditorState,
    opened: bool,
    notes: impl FnOnce() -> Vec<Note>,
) {
    if !(editor.is_editing() && editor.insert_mode()) {
        return state.hide();
    }

    let (line_start, before_cursor, _) = editor.cursor_line();
    match context(line_start, before_cursor) {
        Some(context) if state.tracks(editor, context.anchor) => {
            state.set_query(context.query, editor)
        }
        Some(context) if opened && context.query.is_empty() => {
            state.open(context.anchor, editor.filepath(), notes())
        }
        _ => state.hide(),
    }
}

pub fn update<'a>(
    message: Message,
    state: &mut CompletionState,
    editor: &mut NoteEditorState,
) -> Option<AppMessage<'a>> {
    match message {
        Message::Up => state.move_up(),
        Message::Down => state.move_down(),
        Message::Dismiss => state.hide(),
        Message::Accept => {
            let (replaced, label) = state.selected_insertion()?;
            state.hide();
            insert(editor, replaced, &label);
            return Some(AppMessage::UpdateSelectedNoteContent((
                editor.content.to_string(),
                None,
            )));
        }
    }

    None
}

fn insert(editor: &mut NoteEditorState, replaced: usize, label: &str) {
    (0..replaced).for_each(|_| {
        editor.delete_char();
    });
    label
        .chars()
        .for_each(|character| editor.insert_char(character));

    let (_, _, after_cursor) = editor.cursor_line();
    if after_cursor.starts_with("]]") {
        editor.cursor_right(2);
    } else {
        "]]".chars()
            .for_each(|character| editor.insert_char(character));
    }
}

pub fn handle_key_event(key: KeyEvent, state: &CompletionState) -> Option<Message> {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Esc => Some(Message::Dismiss),
        KeyCode::Up => Some(Message::Up),
        KeyCode::Down => Some(Message::Down),
        KeyCode::Char('p') if control => Some(Message::Up),
        KeyCode::Char('n') if control => Some(Message::Down),
        KeyCode::Enter | KeyCode::Tab if state.has_selection() => Some(Message::Accept),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ratatui::layout::Size;

    use super::*;
    use crate::{
        config::Symbols,
        note_editor::{self, state::EditMode, state::View},
    };

    fn note(name: &str) -> Note {
        Note::new_unchecked(name, Path::new(&format!("/vault/{name}.md")))
    }

    fn notes() -> Vec<Note> {
        ["Daily log", "Project ideas", "Reading list"]
            .into_iter()
            .map(note)
            .collect()
    }

    fn editor(content: &str) -> NoteEditorState<'static> {
        let mut editor = NoteEditorState::new(
            content,
            "test",
            Path::new("/vault/test.md"),
            &Symbols::unicode(),
        );
        editor.resize_viewport(Size::new(60, 10));
        editor.set_view(View::Edit(EditMode::Source));
        editor.set_insert_mode(true);
        editor.jump_to_offset(content.trim_end_matches('\n').len());
        editor
    }

    fn type_text(editor: &mut NoteEditorState, state: &mut CompletionState, text: &str) {
        for character in text.chars() {
            let message = note_editor::Message::KeyEvent(KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::NONE,
            ));
            let opened = opens_completion(&message);
            note_editor::update(message, Size::new(60, 10), editor);
            sync(state, editor, opened, notes);
        }
    }

    fn press(editor: &mut NoteEditorState, state: &mut CompletionState, code: KeyCode) -> bool {
        let key = KeyEvent::new(code, KeyModifiers::NONE);
        match handle_key_event(key, state) {
            Some(message) => {
                update(message, state, editor);
                true
            }
            None => {
                note_editor::update(
                    note_editor::handle_editing_event(key).unwrap(),
                    Size::new(60, 10),
                    editor,
                );
                sync(state, editor, false, notes);
                false
            }
        }
    }

    fn committed(editor: &mut NoteEditorState) -> String {
        editor.exit_insert();
        editor.content.clone()
    }

    #[test]
    fn context_finds_open_brackets_before_cursor() {
        assert_eq!(
            context(10, "see [[Proj"),
            Some(Context {
                anchor: 14,
                query: "Proj"
            })
        );
        assert_eq!(context(0, "see [[Proj]] more"), None);
        assert_eq!(context(0, "see [[Proj|alias"), None);
        assert_eq!(context(0, "no link"), None);
    }

    #[test]
    fn typing_double_bracket_opens_with_all_notes() {
        let mut editor = editor("Hello \n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[");
        assert!(!state.visible);

        type_text(&mut editor, &mut state, "[");
        assert!(state.visible);
        assert_eq!(
            state.labels(),
            vec!["Daily log", "Project ideas", "Reading list"]
        );
    }

    #[test]
    fn typed_text_filters_fuzzily() {
        let mut editor = editor("Link \n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[[rdls");
        assert_eq!(state.labels(), vec!["Reading list"]);
    }

    #[test]
    fn enter_inserts_selected_note_and_closes() {
        let mut editor = editor("See \n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[[proj");
        assert!(press(&mut editor, &mut state, KeyCode::Enter));

        assert!(!state.visible);
        assert!(editor.insert_mode());
        assert_eq!(committed(&mut editor), "See [[Project ideas]]\n");
    }

    #[test]
    fn selection_moves_and_tab_accepts() {
        let mut editor = editor("Link \n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[[");
        press(&mut editor, &mut state, KeyCode::Down);
        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
        update(
            handle_key_event(key, &state).unwrap(),
            &mut state,
            &mut editor,
        );
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        update(
            handle_key_event(key, &state).unwrap(),
            &mut state,
            &mut editor,
        );
        assert!(press(&mut editor, &mut state, KeyCode::Tab));

        assert_eq!(committed(&mut editor), "Link [[Project ideas]]\n");
    }

    #[test]
    fn accept_reuses_existing_closing_brackets() {
        let mut editor = editor("[[]]\n");
        editor.jump_to_offset(2);
        let mut state = CompletionState::default();
        state.open(0, editor.filepath(), notes());

        type_text(&mut editor, &mut state, "daily");
        press(&mut editor, &mut state, KeyCode::Enter);

        assert_eq!(committed(&mut editor), "[[Daily log]]\n");
    }

    #[test]
    fn esc_closes_and_keeps_typed_text() {
        let mut editor = editor("Link \n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[[new");
        assert!(press(&mut editor, &mut state, KeyCode::Esc));
        assert!(!state.visible);
        assert!(editor.insert_mode(), "esc only closes the dropdown");

        type_text(&mut editor, &mut state, " note");
        assert!(!state.visible, "dismissed dropdown stays closed");
        assert_eq!(committed(&mut editor), "Link [[new note\n");
    }

    #[test]
    fn closing_brackets_close_dropdown() {
        let mut editor = editor("Link \n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[[Daily log]");
        assert!(state.visible);
        type_text(&mut editor, &mut state, "]");
        assert!(!state.visible);
    }

    #[test]
    fn deleting_open_brackets_closes_dropdown() {
        let mut editor = editor("Link \n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[[d");
        press(&mut editor, &mut state, KeyCode::Backspace);
        assert!(state.visible, "empty query keeps it open");
        press(&mut editor, &mut state, KeyCode::Backspace);
        assert!(!state.visible);
    }

    #[test]
    fn cursor_leaving_brackets_closes_dropdown() {
        let mut editor = editor("Link ab\n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[[");
        press(&mut editor, &mut state, KeyCode::Left);
        assert!(!state.visible);
    }

    #[test]
    fn alias_pipe_stops_completion() {
        let mut editor = editor("Link \n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[[Daily log|");
        assert!(!state.visible);
    }

    #[test]
    fn enter_without_matches_inserts_newline() {
        let mut editor = editor("Link \n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[[zzzz");
        assert!(state.labels().is_empty());
        assert!(!press(&mut editor, &mut state, KeyCode::Enter));
        assert!(committed(&mut editor).starts_with("Link [[zzzz\n"));
    }

    #[test]
    fn hash_completes_headings_of_current_note() {
        let mut editor = editor("# Intro\n\n## Setup steps\n\nSee \n");
        let mut state = CompletionState::default();

        type_text(&mut editor, &mut state, "[[#set");
        assert_eq!(state.labels(), vec!["Setup steps"]);
        press(&mut editor, &mut state, KeyCode::Enter);

        assert!(committed(&mut editor).ends_with("See [[#Setup steps]]\n"));
    }
}
