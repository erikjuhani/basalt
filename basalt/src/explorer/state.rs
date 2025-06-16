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

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ExplorerState<'a> {
    pub(crate) title: &'a str,
    pub(crate) selected_note: Option<Note>,
    pub(crate) selected_item_index: Option<usize>,
    pub(crate) selected_item_path: Option<PathBuf>,
    pub(crate) items: Vec<Item>,
    pub(crate) flat_items: Vec<(Item, usize)>,
    pub(crate) open: bool,
    pub(crate) sort: Sort,
    pub(crate) list_state: ListState,
    pub(crate) active: bool,
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

impl<'a> ExplorerState<'a> {
    pub fn new(title: &'a str, items: Vec<VaultEntry>) -> Self {
        let items: Vec<Item> = items.into_iter().map(|entry| entry.into()).collect();
        let sort = Sort::default();

        ExplorerState {
            title,
            sort,
            open: true,
            selected_item_index: None,
            selected_item_path: None,
            selected_note: None,
            list_state: ListState::default().with_selected(Some(0)),
            ..Default::default()
        }
        .flatten_with_items(&items)
    }

    pub fn set_active(&self, active: bool) -> Self {
        Self {
            active,
            ..self.clone()
        }
    }

    pub fn toggle(&self) -> Self {
        Self {
            open: !self.open,
            ..self.clone()
        }
    }

    pub fn open(self) -> Self {
        Self { open: true, ..self }
    }

    pub fn close(self) -> Self {
        Self {
            open: false,
            ..self
        }
    }

    pub fn flatten_with_sort(&self, sort: Sort) -> Self {
        let mut items = self.items.clone();
        items.sort_by(sort_items_by(sort));

        Self {
            flat_items: items.iter().flat_map(flatten(sort, 0)).collect(),
            items,
            sort,
            ..self.clone()
        }
    }

    pub fn flatten_with_items(&self, items: &[Item]) -> Self {
        let mut items = items.to_vec();
        items.sort_by(sort_items_by(self.sort));

        Self {
            flat_items: items.iter().flat_map(flatten(self.sort, 0)).collect(),
            items: items.to_vec(),
            ..self.clone()
        }
    }

    pub fn sort(&self) -> Self {
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

    pub fn select(&self) -> Self {
        let Some(selected_item_index) = self.list_state.selected() else {
            return self.clone();
        };

        let Some(current_item) = self.flat_items.get(selected_item_index) else {
            return self.clone();
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
            (Item::File(note), _) => Self {
                selected_note: Some(note.clone()),
                selected_item_index: Some(selected_item_index),
                selected_item_path: Some(note.path.clone()),
                ..self.clone()
            },
        }
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_item_path.clone()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn next(mut self, amount: usize) -> Self {
        let index = self
            .list_state
            .selected()
            .map(|i| (i + amount).min(self.flat_items.len().saturating_sub(1)));

        self.list_state.select(index);

        Self {
            list_state: self.list_state,
            ..self
        }
    }

    pub fn previous(mut self, amount: usize) -> Self {
        let index = self.list_state.selected().map(|i| i.saturating_sub(amount));

        self.list_state.select(index);

        Self {
            list_state: self.list_state,
            ..self
        }
    }
}
