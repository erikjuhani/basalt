use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use basalt_core::obsidian::Note;
use ratatui::widgets::ListState;

use crate::{
    note_editor::{ast::Node, parser, state::NoteEditorState},
    search::fuzzy_rank,
};

const MAX_RESULTS: usize = 50;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::completion) struct Suggestion {
    pub(in crate::completion) label: String,
    pub(in crate::completion) match_ranges: Vec<Range<usize>>,
}

#[derive(Clone, Debug, Default)]
struct Headings {
    note: String,
    headings: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct CompletionState {
    pub visible: bool,
    anchor: usize,
    filepath: PathBuf,
    notes: Vec<Note>,
    headings: Option<Headings>,
    replaced: usize,
    pub(in crate::completion) results: Vec<Suggestion>,
    pub(in crate::completion) list_state: ListState,
}

impl CompletionState {
    pub(in crate::completion) fn open(&mut self, anchor: usize, filepath: &Path, notes: Vec<Note>) {
        if notes.is_empty() {
            return self.hide();
        }

        let mut notes = notes;
        notes.sort_by_key(|note| note.name().to_lowercase());

        *self = Self {
            visible: true,
            anchor,
            filepath: filepath.to_path_buf(),
            notes,
            ..Self::default()
        };
        self.rank_notes("");
    }

    pub fn hide(&mut self) {
        *self = Self::default();
    }

    pub fn is_active_for(&self, editor: &NoteEditorState) -> bool {
        self.visible
            && editor.is_editing()
            && editor.insert_mode()
            && self.filepath == editor.filepath()
    }

    pub(in crate::completion) fn tracks(&self, editor: &NoteEditorState, anchor: usize) -> bool {
        self.is_active_for(editor) && self.anchor == anchor
    }

    pub(in crate::completion) fn has_selection(&self) -> bool {
        self.list_state.selected().is_some()
    }

    pub(in crate::completion) fn selected_insertion(&self) -> Option<(usize, String)> {
        let suggestion = self.results.get(self.list_state.selected()?)?;
        Some((self.replaced, suggestion.label.clone()))
    }

    pub(in crate::completion) fn set_query(&mut self, query: &str, editor: &NoteEditorState) {
        match query.split_once('#') {
            Some((note, heading)) => {
                self.load_headings(note, editor);
                self.rank_headings(heading);
            }
            None => self.rank_notes(query),
        }
    }

    fn load_headings(&mut self, note: &str, editor: &NoteEditorState) {
        let note = note.to_lowercase();
        if self
            .headings
            .as_ref()
            .is_some_and(|loaded| loaded.note == note)
        {
            return;
        }

        let content = if note.is_empty() {
            Some(editor.content.to_string())
        } else {
            self.notes
                .iter()
                .find(|candidate| candidate.name().to_lowercase() == note)
                .and_then(|found| std::fs::read_to_string(found.path()).ok())
        };

        self.headings = Some(Headings {
            note,
            headings: content.as_deref().map(headings).unwrap_or_default(),
        });
    }

    fn rank_notes(&mut self, query: &str) {
        self.replaced = query.chars().count();
        self.results = if query.is_empty() {
            self.notes
                .iter()
                .take(MAX_RESULTS)
                .map(|note| Suggestion {
                    label: note.name().to_string(),
                    match_ranges: Vec::new(),
                })
                .collect()
        } else {
            rank(query, &self.notes, |note| note.name())
        };
        self.select_first();
    }

    fn rank_headings(&mut self, query: &str) {
        self.replaced = query.chars().count();
        let headings = self
            .headings
            .as_ref()
            .map(|loaded| loaded.headings.as_slice())
            .unwrap_or_default();
        self.results = if query.is_empty() {
            headings
                .iter()
                .take(MAX_RESULTS)
                .map(|heading| Suggestion {
                    label: heading.clone(),
                    match_ranges: Vec::new(),
                })
                .collect()
        } else {
            rank(query, headings, String::as_str)
        };
        self.select_first();
    }

    fn select_first(&mut self) {
        self.list_state
            .select((!self.results.is_empty()).then_some(0));
    }

    pub(in crate::completion) fn move_down(&mut self) {
        if let Some(row) = self.list_state.selected() {
            self.list_state
                .select(Some((row + 1).min(self.results.len().saturating_sub(1))));
        }
    }

    pub(in crate::completion) fn move_up(&mut self) {
        if let Some(row) = self.list_state.selected() {
            self.list_state.select(Some(row.saturating_sub(1)));
        }
    }

    #[cfg(test)]
    pub(in crate::completion) fn labels(&self) -> Vec<&str> {
        self.results
            .iter()
            .map(|suggestion| suggestion.label.as_str())
            .collect()
    }
}

fn rank<T>(query: &str, candidates: &[T], text: impl Fn(&T) -> &str) -> Vec<Suggestion> {
    fuzzy_rank(query, candidates, &text, MAX_RESULTS)
        .into_iter()
        .map(|(index, match_ranges)| Suggestion {
            label: text(&candidates[index]).to_string(),
            match_ranges,
        })
        .collect()
}

fn headings(content: &str) -> Vec<String> {
    parser::from_str(content)
        .into_iter()
        .filter_map(|node| match node {
            Node::Heading { text, .. } => Some(text.to_string().trim().to_string()),
            _ => None,
        })
        .filter(|heading| !heading.is_empty())
        .collect()
}
