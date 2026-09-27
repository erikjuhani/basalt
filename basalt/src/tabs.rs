use std::{collections::HashMap, path::Path};

use crate::{app::SelectedNote, config::Theme, note_editor::state::NoteEditorState};

#[derive(Clone)]
pub enum TabEditor<'a> {
    Pending,
    Loaded(Box<NoteEditorState<'a>>),
}

impl<'a> TabEditor<'a> {
    fn loaded(&self) -> Option<&NoteEditorState<'a>> {
        match self {
            TabEditor::Loaded(editor) => Some(editor),
            TabEditor::Pending => None,
        }
    }

    fn loaded_mut(&mut self) -> Option<&mut NoteEditorState<'a>> {
        match self {
            TabEditor::Loaded(editor) => Some(editor),
            TabEditor::Pending => None,
        }
    }

    fn modified(&self) -> bool {
        self.loaded().is_some_and(|editor| editor.modified())
    }
}

#[derive(Clone)]
pub struct Tab<'a> {
    pub note: SelectedNote,
    pub editor: TabEditor<'a>,
}

#[derive(Default, Clone)]
pub struct Tabs<'a> {
    tabs: Vec<Tab<'a>>,
    active: usize,
}

impl<'a> Tabs<'a> {
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_note(&self) -> Option<&SelectedNote> {
        self.tabs.get(self.active).map(|tab| &tab.note)
    }

    pub fn notes(&self) -> impl Iterator<Item = &SelectedNote> {
        self.tabs.iter().map(|tab| &tab.note)
    }

    pub fn active_note_mut(&mut self) -> Option<&mut SelectedNote> {
        self.tabs.get_mut(self.active).map(|tab| &mut tab.note)
    }

    pub fn active_editor(&self) -> Option<&NoteEditorState<'a>> {
        self.tabs
            .get(self.active)
            .and_then(|tab| tab.editor.loaded())
    }

    pub fn active_editor_mut(&mut self) -> Option<&mut NoteEditorState<'a>> {
        self.tabs
            .get_mut(self.active)
            .and_then(|tab| tab.editor.loaded_mut())
    }

    pub fn active_is_pending(&self) -> bool {
        self.tabs
            .get(self.active)
            .is_some_and(|tab| matches!(tab.editor, TabEditor::Pending))
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab<'a>> {
        self.tabs.get_mut(self.active)
    }

    pub fn pending_notes(&self) -> impl Iterator<Item = &SelectedNote> {
        self.tabs
            .iter()
            .filter(|tab| matches!(tab.editor, TabEditor::Pending))
            .map(|tab| &tab.note)
    }

    pub fn tab_mut(&mut self, path: &Path) -> Option<&mut Tab<'a>> {
        self.tabs.iter_mut().find(|tab| tab.note.path() == path)
    }

    /// Applies a theme to every loaded tab's editor so switching tabs never
    /// reveals a stale palette.
    pub fn set_theme(&mut self, theme: &Theme) {
        for tab in &mut self.tabs {
            if let Some(editor) = tab.editor.loaded_mut() {
                editor.set_theme(theme);
            }
        }
    }

    fn index_of(&self, path: &Path) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.note.path() == path)
    }

    pub fn open_or_focus(&mut self, path: &Path) -> bool {
        match self.index_of(path) {
            Some(index) => {
                self.active = index;
                true
            }
            None => false,
        }
    }

    /// Appends a pending tab without changing focus.
    pub fn open_pending(&mut self, note: SelectedNote) {
        self.tabs.push(Tab {
            note,
            editor: TabEditor::Pending,
        });
    }

    pub fn open(&mut self, tab: Tab<'a>) {
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    pub fn next(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
        }
    }

    pub fn close_active(&mut self) {
        if self.active < self.tabs.len() {
            self.tabs.remove(self.active);
            self.active = self.active.min(self.tabs.len().saturating_sub(1));
        }
    }

    pub fn rename(&mut self, old: &Path, new: &Path, name: &str) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.note.path() == old) {
            tab.note.set_path(new);
            tab.note.set_name(name);
            if let Some(editor) = tab.editor.loaded_mut() {
                editor.set_filepath(new);
                editor.set_filename(name);
            }
        }
    }

    pub(crate) fn titles(&self) -> Vec<(String, bool, bool)> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for tab in &self.tabs {
            *counts.entry(tab.note.name()).or_default() += 1;
        }
        self.tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let name = tab.note.name();
                let label = if counts[name] > 1 {
                    tab.note
                        .path()
                        .parent()
                        .and_then(|parent| parent.file_name())
                        .and_then(|dir| dir.to_str())
                        .map(|dir| format!("{dir}/{name}"))
                        .unwrap_or_else(|| name.to_string())
                } else {
                    name.to_string()
                };
                (label, index == self.active, tab.editor.modified())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::config::Symbols;

    fn tab(name: &str) -> Tab<'static> {
        tab_in("", name)
    }

    fn tab_in(dir: &str, name: &str) -> Tab<'static> {
        let path = PathBuf::from(format!("/vault/{dir}/{name}.md"));
        let editor = NoteEditorState::new("", name, &path, &Symbols::unicode());
        Tab {
            note: SelectedNote::new(name, &path, ""),
            editor: TabEditor::Loaded(Box::new(editor)),
        }
    }

    fn pending_note(name: &str) -> SelectedNote {
        let path = PathBuf::from(format!("/vault/{name}.md"));
        SelectedNote::new(name, &path, "")
    }

    #[test]
    fn open_focuses_new_tab() {
        let mut tabs = Tabs::default();
        tabs.open(tab("a"));
        tabs.open(tab("b"));
        assert_eq!(tabs.active_note().map(SelectedNote::name), Some("b"));
    }

    #[test]
    fn pending_notes_skips_loaded_tabs() {
        let mut tabs = Tabs::default();
        tabs.open(tab("a"));
        tabs.open_pending(pending_note("b"));
        tabs.open_pending(pending_note("c"));

        let names: Vec<&str> = tabs.pending_notes().map(SelectedNote::name).collect();

        assert_eq!(names, vec!["b", "c"]);
    }

    #[test]
    fn tab_mut_hydrates_a_pending_tab_by_path() {
        let mut tabs = Tabs::default();
        tabs.open_pending(pending_note("a"));

        let tab = tabs.tab_mut(Path::new("/vault/a.md")).unwrap();
        let editor = NoteEditorState::new("", "a", tab.note.path(), &Symbols::unicode());
        tab.editor = TabEditor::Loaded(Box::new(editor));

        assert!(tabs.pending_notes().next().is_none());
        assert!(tabs.active_editor().is_some());
        assert!(tabs.tab_mut(Path::new("/vault/missing.md")).is_none());
    }

    #[test]
    fn open_or_focus_reuses_open_tab() {
        let mut tabs = Tabs::default();
        tabs.open(tab("a"));
        tabs.open(tab("b"));

        assert!(tabs.open_or_focus(Path::new("/vault/a.md")));
        assert_eq!(tabs.active_note().map(SelectedNote::name), Some("a"));
        assert!(!tabs.open_or_focus(Path::new("/vault/c.md")));
    }

    #[test]
    fn next_and_prev_wrap_around() {
        let mut tabs = Tabs::default();
        tabs.open(tab("a"));
        tabs.open(tab("b"));

        tabs.next();
        assert_eq!(tabs.active_note().map(SelectedNote::name), Some("a"));
        tabs.prev();
        assert_eq!(tabs.active_note().map(SelectedNote::name), Some("b"));
    }

    #[test]
    fn close_active_clamps_focus() {
        let mut tabs = Tabs::default();
        tabs.open(tab("a"));
        tabs.open(tab("b"));

        tabs.close_active();
        assert_eq!(tabs.active_note().map(SelectedNote::name), Some("a"));
        tabs.close_active();
        assert!(tabs.is_empty());
        assert_eq!(tabs.active_note(), None);
    }

    #[test]
    fn open_pending_appends_without_changing_focus() {
        let mut tabs = Tabs::default();
        tabs.open(tab("a"));

        tabs.open_pending(pending_note("b"));

        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs.active_note().map(SelectedNote::name), Some("a"));
        assert!(!tabs.active_is_pending());
    }

    #[test]
    fn next_can_land_on_a_pending_tab() {
        let mut tabs = Tabs::default();
        tabs.open(tab("a"));
        tabs.open_pending(pending_note("b"));

        tabs.next();

        assert_eq!(tabs.active_note().map(SelectedNote::name), Some("b"));
        assert!(tabs.active_is_pending());
        assert!(tabs.active_editor().is_none());
    }

    #[test]
    fn active_tab_mut_loads_a_pending_tab() {
        let mut tabs = Tabs::default();
        tabs.open_pending(pending_note("a"));

        let tab = tabs.active_tab_mut().unwrap();
        let editor = NoteEditorState::new("", "a", tab.note.path(), &Symbols::unicode());
        tab.editor = TabEditor::Loaded(Box::new(editor));

        assert!(!tabs.active_is_pending());
        assert!(tabs.active_editor().is_some());
    }

    #[test]
    fn close_active_can_leave_a_pending_tab_focused() {
        let mut tabs = Tabs::default();
        tabs.open(tab("a"));
        tabs.open_pending(pending_note("b"));
        tabs.next();

        tabs.close_active();

        assert_eq!(tabs.active_note().map(SelectedNote::name), Some("a"));
        assert!(!tabs.active_is_pending());
    }

    #[test]
    fn titles_report_pending_tabs_as_unmodified() {
        let mut tabs = Tabs::default();
        tabs.open_pending(pending_note("a"));

        let titles = tabs.titles();

        assert_eq!(titles, [("a".to_string(), true, false)]);
    }

    #[test]
    fn same_name_tabs_are_disambiguated_by_parent() {
        let mut tabs = Tabs::default();
        tabs.open(tab_in("alpha", "note"));
        tabs.open(tab_in("beta", "note"));
        tabs.open(tab_in("gamma", "unique"));

        let labels: Vec<String> = tabs.titles().into_iter().map(|(label, ..)| label).collect();
        assert_eq!(labels, ["alpha/note", "beta/note", "unique"]);
    }
}
