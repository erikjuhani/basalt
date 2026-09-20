use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

use basalt_core::obsidian::{Note, Vault, VaultEntry};
use frizbee::{iter::FuzzyMatchExt, radix_sort_matches, Config, Match, Matching};
use ratatui::widgets::ListState;
use rayon::prelude::*;

use crate::input::TextInput;

const MAX_RESULTS: usize = 200;
/// Skip lines longer than this, in bytes. A markdown paragraph is one long
/// unwrapped line, so the cap must clear normal prose; it exists only to drop
/// pathological lines (embedded data URIs, minified blobs) that would slow the
/// fuzzy match without being real search targets.
const MAX_LINE: usize = 8192;
const INDEX_BATCH: usize = 256;

/// The searchable lines, filled in batches while indexing runs so results show
/// before the whole vault is read. `complete` marks the final batch.
#[derive(Debug, Default)]
pub(in crate::search) struct SearchIndex {
    notes: Vec<Note>,
    lines: Vec<Candidate>,
    complete: bool,
}

#[derive(Debug)]
struct Candidate {
    note_index: usize,
    line_index: usize,
    text: String,
}

#[derive(Clone, Debug)]
pub(in crate::search) struct SearchResult {
    pub(in crate::search) note: Note,
    line_index: usize,
    pub(in crate::search) text: String,
    pub(in crate::search) match_ranges: Vec<Range<usize>>,
}

#[derive(Debug, PartialEq)]
pub(in crate::search) struct Target<'a> {
    pub(in crate::search) note: &'a Note,
    pub(in crate::search) line: usize,
    pub(in crate::search) column: usize,
}

pub(in crate::search) type SharedIndex = Arc<Mutex<SearchIndex>>;

#[derive(Debug, Default, Clone)]
pub struct SearchState {
    pub visible: bool,
    pub(in crate::search) query: TextInput,
    pub(in crate::search) index: SharedIndex,
    pub(in crate::search) indexed: bool,
    pub(in crate::search) results: Vec<SearchResult>,
    pub(in crate::search) total: usize,
    pub(in crate::search) list_state: ListState,
}

impl SearchState {
    pub(in crate::search) fn toggle(&mut self, vault: &Vault) {
        if self.visible {
            return self.hide();
        }

        self.visible = true;

        let index = Arc::clone(&self.index);
        let vault = vault.clone();
        std::thread::spawn(move || load_index(&index, &vault));
    }

    pub(in crate::search) fn hide(&mut self) {
        *self = Self::default();
    }

    pub(in crate::search) fn indexing(&self) -> bool {
        !self.index.lock().unwrap().complete
    }

    pub(in crate::search) fn poll(&mut self) {
        if self.indexed {
            return;
        }

        self.refresh();
        if self.index.lock().unwrap().complete {
            self.indexed = true;
        }
    }

    pub(in crate::search) fn selected_target(&self) -> Option<Target<'_>> {
        self.list_state
            .selected()
            .and_then(|row| self.results.get(row))
            .map(|result| Target {
                note: &result.note,
                line: result.line_index,
                column: result.match_ranges.first().map_or(0, |range| range.start),
            })
    }

    fn edit(&mut self, edit: impl FnOnce(&mut TextInput)) {
        edit(&mut self.query);
        self.refresh();
    }

    pub(in crate::search) fn insert_char(&mut self, character: char) {
        self.edit(|query| query.insert_char(character));
    }

    pub(in crate::search) fn delete_char(&mut self) {
        self.edit(TextInput::delete_char);
    }

    pub(in crate::search) fn delete_char_forward(&mut self) {
        self.edit(TextInput::delete_char_forward);
    }

    pub(in crate::search) fn delete_word(&mut self) {
        self.edit(TextInput::delete_word);
    }

    pub(in crate::search) fn delete_word_forward(&mut self) {
        self.edit(TextInput::delete_word_forward);
    }

    pub(in crate::search) fn delete_to_line_start(&mut self) {
        self.edit(TextInput::delete_to_line_start);
    }

    pub(in crate::search) fn delete_to_line_end(&mut self) {
        self.edit(TextInput::delete_to_line_end);
    }

    pub(in crate::search) fn cursor_left(&mut self) {
        self.query.cursor_left(1);
    }

    pub(in crate::search) fn cursor_right(&mut self) {
        self.query.cursor_right(1);
    }

    pub(in crate::search) fn cursor_line_start(&mut self) {
        self.query.cursor_line_start();
    }

    pub(in crate::search) fn cursor_line_end(&mut self) {
        self.query.cursor_line_end();
    }

    pub(in crate::search) fn cursor_word_forward(&mut self) {
        self.query.cursor_word_forward();
    }

    pub(in crate::search) fn cursor_word_backward(&mut self) {
        self.query.cursor_word_backward();
    }

    pub(in crate::search) fn move_down(&mut self) {
        if self.results.is_empty() {
            return;
        }

        let next = self
            .list_state
            .selected()
            .map_or(0, |row| (row + 1).min(self.results.len() - 1));

        self.list_state.select(Some(next));
    }

    pub(in crate::search) fn move_up(&mut self) {
        if self.results.is_empty() {
            return;
        }

        let previous = self
            .list_state
            .selected()
            .map_or(0, |row| row.saturating_sub(1));

        self.list_state.select(Some(previous));
    }

    fn refresh(&mut self) {
        let query = self.query.value();
        let (results, total) = {
            let index = self.index.lock().unwrap();
            let results = if query.is_empty() {
                Vec::new()
            } else {
                rank(query, &index)
            };
            (results, index.lines.len())
        };

        self.results = results;
        self.total = total;
        self.list_state
            .select((!self.results.is_empty()).then_some(0));
    }
}

fn load_index(index: &SharedIndex, vault: &Vault) {
    fn walk(entries: Vec<VaultEntry>, notes: &mut Vec<Note>) {
        for entry in entries {
            match entry {
                VaultEntry::File(note) if is_markdown(note.path()) => notes.push(note),
                VaultEntry::File(_) => {}
                VaultEntry::Directory { entries, .. } => walk(entries, notes),
            }
        }
    }

    let mut notes = Vec::new();
    walk(vault.entries(), &mut notes);

    for batch in notes.chunks(INDEX_BATCH) {
        let read: Vec<(Note, Vec<(usize, String)>)> = batch
            .par_iter()
            .map(|note| {
                let content = std::fs::read_to_string(note.path()).unwrap_or_default();
                (note.clone(), split_lines(&content).collect())
            })
            .collect();

        extend_index(&mut index.lock().unwrap(), read);
    }

    index.lock().unwrap().complete = true;
}

fn extend_index(index: &mut SearchIndex, batch: Vec<(Note, Vec<(usize, String)>)>) {
    for (note, lines) in batch {
        let note_index = index.notes.len();
        index
            .lines
            .extend(lines.into_iter().map(|(line_index, text)| Candidate {
                note_index,
                line_index,
                text,
            }));
        index.notes.push(note);
    }
}

fn is_markdown(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

fn split_lines(content: &str) -> impl Iterator<Item = (usize, String)> + '_ {
    content
        .lines()
        .enumerate()
        .filter(|(_, text)| text.len() <= MAX_LINE)
        .map(|(line_index, text)| (line_index, text.to_string()))
}

fn rank(query: &str, index: &SearchIndex) -> Vec<SearchResult> {
    let config = Config::default().matching(Matching::Fuzzy);

    let mut matches: Vec<Match> = index
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .fuzzy_match(query, &config)
        .collect();
    radix_sort_matches(&mut matches);
    matches.truncate(MAX_RESULTS);

    let survivors: Vec<&str> = matches
        .iter()
        .map(|matched| index.lines[matched.index as usize].text.as_str())
        .collect();

    survivors
        .iter()
        .copied()
        .fuzzy_match_indices(query, &config)
        .map(|found| {
            let line = &index.lines[matches[found.index as usize].index as usize];
            SearchResult {
                note: index.notes[line.note_index].clone(),
                line_index: line.line_index,
                text: line.text.clone(),
                match_ranges: coalesce(&found.indices),
            }
        })
        .collect()
}

fn coalesce(indices: &[u32]) -> Vec<Range<usize>> {
    let mut positions: Vec<usize> = indices.iter().map(|&index| index as usize).collect();
    positions.sort_unstable();
    positions.dedup();

    positions
        .chunk_by(|a, b| *b == *a + 1)
        .map(|run| run[0]..run[run.len() - 1] + 1)
        .collect()
}

#[cfg(test)]
pub(in crate::search) fn make_index(notes: Vec<(Note, String)>) -> SearchIndex {
    let mut index = SearchIndex {
        complete: true,
        ..Default::default()
    };
    let batch = notes
        .into_iter()
        .map(|(note, content)| (note, split_lines(&content).collect()))
        .collect();
    extend_index(&mut index, batch);
    index
}

#[cfg(test)]
pub(in crate::search) fn loaded_index(notes: Vec<(Note, String)>) -> SharedIndex {
    Arc::new(Mutex::new(make_index(notes)))
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    #[test]
    fn coalesce_sorts_reversed_indices_and_groups_runs() {
        assert_eq!(coalesce(&[3, 1, 0]), vec![0..2, 3..4]);
    }

    #[test]
    fn fuzzy_ranks_and_highlights_scattered_matches() {
        let index = make_index(vec![(
            Note::new_unchecked("n", std::path::Path::new("/n.md")),
            "main.rs\nreadme".to_string(),
        )]);

        let results = rank("mn", &index);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "main.rs");
        assert_eq!(results[0].match_ranges, vec![0..1, 3..4]);
    }

    #[test]
    fn best_match_leads_and_is_preselected() {
        let mut state = SearchState {
            index: loaded_index(vec![(
                Note::new_unchecked("notes", std::path::Path::new("/notes.md")),
                "unrelated tail\nthe main entry".to_string(),
            )]),
            ..SearchState::default()
        };
        "main"
            .chars()
            .for_each(|character| state.insert_char(character));

        assert_eq!(state.results.len(), 1);
        assert_eq!(
            state.selected_target(),
            Some(Target {
                note: &state.results[0].note,
                line: 1,
                column: 4,
            })
        );
    }

    #[test]
    fn empty_query_yields_no_hits() {
        let mut state = SearchState {
            index: loaded_index(vec![(
                Note::new_unchecked("a", std::path::Path::new("/a.md")),
                "find".to_string(),
            )]),
            ..SearchState::default()
        };
        state.refresh();
        assert!(state.results.is_empty());
        assert_eq!(state.selected_target(), None);
    }
}
