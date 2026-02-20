use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use basalt_core::obsidian::{Note, VaultEntry};
use ratatui::widgets::ListState;

use super::Item;

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum Sort {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum Visibility {
    Hidden,
    #[default]
    Visible,
    FullWidth,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ExplorerState {
    pub(crate) title: String,
    pub(crate) selected_note: Option<Note>,
    pub(crate) selected_item_index: Option<usize>,
    pub(crate) selected_item_path: Option<PathBuf>,
    pub(crate) items: Vec<Item>,
    pub(crate) all_flat_items: Vec<(Item, usize)>,
    pub(crate) flat_items: Vec<(Item, usize)>,
    pub(crate) visibility: Visibility,
    pub(crate) active: bool,
    pub(crate) sort: Sort,
    pub(crate) list_state: ListState,
    pub(crate) filter_query: Option<String>,

    pub(crate) editing: bool,
}

fn fuzzy_match_score(haystack: &str, needle: &str) -> Option<usize> {
    let haystack_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();

    if let Some(index) = haystack_lower.find(&needle_lower) {
        return Some(10_000usize.saturating_sub(index));
    }

    let mut score = 0usize;
    let mut cursor = 0usize;
    let chars = haystack_lower.chars().collect::<Vec<_>>();

    for target in needle_lower.chars() {
        let mut found = None;
        for (index, c) in chars.iter().enumerate().skip(cursor) {
            if *c == target {
                found = Some(index);
                break;
            }
        }

        let index = found?;
        score += 100usize.saturating_sub(index.saturating_sub(cursor));
        cursor = index + 1;
    }

    Some(score)
}

/// Calculates the vertical offset of list items in rows.
///
/// When the selected item is near the end of the list and there aren't enough items
/// remaining to keep the selection vertically centered, we shift the offset to show
/// as many trailing items as possible instead of centering the selection.
///
/// This prevents empty lines from appearing at the bottom of the list when the
/// selection moves toward the end.
///
/// Without this check, you'd see output like:
/// ╭────────╮
/// │ 3 item │
/// │>4 item │
/// │ 5 item │
/// │        │
/// ╰────────╯
///
/// With this check, the list scrolls up to fill the remaining space:
/// ╭────────╮
/// │ 2 item │
/// │ 3 item │
/// │>4 item │
/// │ 5 item │
/// ╰────────╯
///
/// The goal is to avoid showing unnecessary blank rows and to maximize visible items.
fn calculate_offset(row: usize, items_count: usize, window_height: usize) -> usize {
    let half = window_height / 2;

    if row + half > items_count.saturating_sub(1) {
        items_count.saturating_sub(window_height)
    } else {
        row.saturating_sub(half)
    }
}

pub fn flatten(sort: Sort, depth: usize) -> impl Fn(&Item) -> Vec<(Item, usize)> {
    move |item| match item {
        Item::File(..) => vec![(item.clone(), depth)],
        Item::Directory {
            expanded: true,
            items,
            ..
        } => [(item.clone(), depth)]
            .into_iter()
            .chain({
                let mut items = items.clone();
                items.sort_by(sort_items_by(sort));
                items
                    .iter()
                    .flat_map(flatten(sort, depth + 1))
                    .collect::<Vec<_>>()
            })
            .collect(),
        Item::Directory {
            expanded: false, ..
        } => [(item.clone(), depth)].to_vec(),
    }
}

fn sort_items_by(sort: Sort) -> impl Fn(&Item, &Item) -> Ordering {
    move |a, b| match (a.is_dir(), b.is_dir()) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (true, true) => Ordering::Equal,
        _ => {
            let a = a.name().to_lowercase();
            let b = b.name().to_lowercase();
            match sort {
                Sort::Asc => a.cmp(&b),
                Sort::Desc => b.cmp(&a),
            }
        }
    }
}

impl ExplorerState {
    pub fn new(title: &str, items: Vec<VaultEntry>) -> Self {
        let items: Vec<Item> = items.into_iter().map(|entry| entry.into()).collect();
        let sort = Sort::default();

        let mut state = ExplorerState {
            title: title.to_string(),
            sort,
            active: true,
            visibility: Visibility::Visible,
            selected_item_index: None,
            selected_item_path: None,
            selected_note: None,
            list_state: ListState::default().with_selected(Some(0)),
            ..Default::default()
        };

        state.flatten_with_items(&items);
        state
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    fn map_to_item(&self, entry: VaultEntry) -> Item {
        match entry {
            VaultEntry::Directory {
                name,
                path,
                entries,
            } => {
                let expanded = self
                    .flat_items
                    .iter()
                    .find_map(|(item, _)| match item {
                        Item::Directory {
                            path: item_path,
                            expanded,
                            ..
                        } if &path == item_path => Some(*expanded),
                        _ => None,
                    })
                    .unwrap_or(false);

                Item::Directory {
                    name,
                    path,
                    expanded,
                    items: entries
                        .into_iter()
                        .map(|entry| self.map_to_item(entry))
                        .collect(),
                }
            }
            _ => entry.into(),
        }
    }

    pub fn with_entries(&mut self, entries: Vec<VaultEntry>, rename: Option<(PathBuf, PathBuf)>) {
        let items: Vec<Item> = entries
            .into_iter()
            .map(|entry| self.map_to_item(entry))
            .collect();

        self.flatten_with_items(&items);

        if let Some((original_path, new_path)) = rename {
            if let Some(index) = self.flat_items.iter().position(|(item, _)| match item {
                Item::File(note) => note.path() == new_path,
                Item::Directory { path: dir_path, .. } => dir_path == &new_path,
            }) {
                self.list_state.select(Some(index));

                // Only update selection if the renamed item was the previously selected item
                if self.selected_item_path.as_ref() == Some(&original_path) {
                    self.selected_item_index = Some(index);
                    self.selected_item_path = Some(new_path);
                }
            }
        }
    }

    pub fn hide_pane(&mut self) {
        match self.visibility {
            Visibility::FullWidth => self.visibility = Visibility::Visible,
            Visibility::Visible => self.visibility = Visibility::Hidden,
            _ => {}
        }
    }

    pub fn expand_pane(&mut self) {
        match self.visibility {
            Visibility::Hidden => self.visibility = Visibility::Visible,
            Visibility::Visible => self.visibility = Visibility::FullWidth,
            _ => {}
        }
    }

    pub fn toggle(&mut self) {
        if self.is_open() {
            self.visibility = Visibility::Hidden;
        } else {
            self.visibility = Visibility::Visible;
        }
    }

    pub fn flatten_with_sort(&mut self, sort: Sort) {
        let mut items = self.items.clone();
        items.sort_by(sort_items_by(sort));

        self.all_flat_items = items.iter().flat_map(flatten(sort, 0)).collect();
        self.flat_items = self.all_flat_items.clone();
        self.items = items;
        self.sort = sort;
        self.apply_filter_view();
    }

    pub fn flatten_with_items(&mut self, items: &[Item]) {
        let mut items = items.to_vec();
        items.sort_by(sort_items_by(self.sort));

        self.all_flat_items = items.iter().flat_map(flatten(self.sort, 0)).collect();
        self.flat_items = self.all_flat_items.clone();
        self.items = items.to_vec();
        self.apply_filter_view();
    }

    fn apply_filter_view(&mut self) {
        if let Some(query) = &self.filter_query {
            let mut filtered = self
                .all_flat_items
                .iter()
                .filter_map(|(item, depth)| match item {
                    Item::File(note) => fuzzy_match_score(note.name(), query)
                        .map(|score| (item.clone(), *depth, score)),
                    _ => None,
                })
                .collect::<Vec<_>>();

            filtered.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.name().cmp(b.0.name())));
            self.flat_items = filtered
                .into_iter()
                .map(|(item, depth, _)| (item, depth))
                .collect();
        } else {
            self.flat_items = self.all_flat_items.clone();
        }

        if self.flat_items.is_empty() {
            self.list_state.select(None);
        } else if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }
    }

    pub fn apply_filter(&mut self, query: String) {
        let query = query.trim().to_string();

        if query.is_empty() {
            self.clear_filter();
            return;
        }

        self.filter_query = Some(query);
        self.apply_filter_view();
        self.list_state.select(if self.flat_items.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    pub fn clear_filter(&mut self) {
        self.filter_query = None;
        self.apply_filter_view();
    }

    pub fn filter_query(&self) -> Option<&str> {
        self.filter_query.as_deref()
    }

    pub fn sort(&mut self) {
        let sort = match self.sort {
            Sort::Asc => Sort::Desc,
            Sort::Desc => Sort::Asc,
        };

        self.flatten_with_sort(sort)
    }

    pub fn update_offset_mut(&mut self, window_height: usize) -> &Self {
        if !self.items.is_empty() {
            let idx = self.list_state.selected().unwrap_or_default();
            let items_count = self.items.len();

            let offset = calculate_offset(idx, items_count, window_height);

            let list_state = &mut self.list_state;
            *list_state.offset_mut() = offset;
        }

        self
    }

    fn toggle_item_in_tree(item: &Item, identifier: &Path) -> Item {
        let item = item.clone();

        match item {
            Item::Directory {
                expanded,
                path,
                name,
                items,
            } => {
                let expanded = if path == identifier {
                    !expanded
                } else {
                    expanded
                };

                Item::Directory {
                    name,
                    path,
                    expanded,
                    items: items
                        .iter()
                        .map(|child| Self::toggle_item_in_tree(child, identifier))
                        .collect(),
                }
            }
            _ => item,
        }
    }

    pub fn select(&mut self) {
        let Some(selected_item_index) = self.list_state.selected() else {
            return;
        };

        let Some(current_item) = self.flat_items.get(selected_item_index) else {
            return;
        };

        match current_item {
            (Item::Directory { path, .. }, _) => {
                let items: Vec<Item> = self
                    .items
                    .clone()
                    .iter()
                    .map(|item| Self::toggle_item_in_tree(item, path))
                    .collect();

                self.flatten_with_items(&items)
            }
            (Item::File(note), _) => {
                self.selected_note = Some(note.clone());
                self.selected_item_index = Some(selected_item_index);
                self.selected_item_path = Some(note.path().to_path_buf());
            }
        }
    }

    pub fn current_item(&self) -> Option<&Item> {
        let selected_item_index = self.list_state.selected()?;
        self.flat_items
            .get(selected_item_index)
            .map(|(item, _)| item)
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_item_path.clone()
    }

    pub fn is_open(&self) -> bool {
        matches!(self.visibility, Visibility::Visible | Visibility::FullWidth)
    }

    pub fn next(&mut self, amount: usize) {
        let index = self
            .list_state
            .selected()
            .map(|i| (i + amount).min(self.flat_items.len().saturating_sub(1)));

        self.list_state.select(index);
    }

    pub fn previous(&mut self, amount: usize) {
        let index = self.list_state.selected().map(|i| i.saturating_sub(amount));

        self.list_state.select(index);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use basalt_core::obsidian::{Note, VaultEntry};

    use super::ExplorerState;

    #[test]
    fn test_apply_filter_reduces_to_matching_notes() {
        let entries = vec![
            VaultEntry::File(Note::new_unchecked("Alpha", Path::new("Alpha.md"))),
            VaultEntry::File(Note::new_unchecked("Beta", Path::new("Beta.md"))),
            VaultEntry::File(Note::new_unchecked("Gamma", Path::new("Gamma.md"))),
        ];

        let mut state = ExplorerState::new("Test", entries);
        state.apply_filter("ga".to_string());

        assert_eq!(state.flat_items.len(), 1);
        assert_eq!(state.flat_items[0].0.name(), "Gamma");
        assert_eq!(state.filter_query(), Some("ga"));
    }

    #[test]
    fn test_apply_filter_empty_clears_filter() {
        let entries = vec![
            VaultEntry::File(Note::new_unchecked("Alpha", Path::new("Alpha.md"))),
            VaultEntry::File(Note::new_unchecked("Beta", Path::new("Beta.md"))),
        ];

        let mut state = ExplorerState::new("Test", entries);
        let all = state.flat_items.len();

        state.apply_filter("alp".to_string());
        assert!(state.flat_items.len() < all);

        state.apply_filter(" ".to_string());
        assert_eq!(state.flat_items.len(), all);
        assert_eq!(state.filter_query(), None);
    }
}
