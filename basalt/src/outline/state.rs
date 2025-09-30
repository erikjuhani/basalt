use std::{iter::Peekable, ops::Range, slice::Iter};

use ratatui::widgets::ListState;

use crate::note_editor::ast::{HeadingLevel, Node};

use super::item::{FindItem, Flatten, Item};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct OutlineState {
    pub(crate) selected_item_index: Option<usize>,
    pub(crate) items: Vec<Item>,
    pub(crate) open: bool,
    pub(crate) list_state: ListState,
    pub(crate) active: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Heading {
    index: usize,
    level: HeadingLevel,
    content: String,
}

#[derive(Debug, Clone, PartialEq)]
struct HeadingEntry {
    range: Range<usize>,
    level: HeadingLevel,
    content: String,
    children: Vec<HeadingEntry>,
}

impl From<HeadingEntry> for Item {
    fn from(value: HeadingEntry) -> Self {
        if value.children.is_empty() {
            Item::Heading {
                range: value.range,
                content: value.content,
            }
        } else {
            Item::HeadingEntry {
                range: value.range,
                content: value.content,
                children: value.children.into_iter().map(Item::from).collect(),
                expanded: false,
            }
        }
    }
}

fn build_outline_tree(headings: &[Heading], max_end: usize) -> Vec<HeadingEntry> {
    fn build_outline_tree_rec(
        headings: &mut Peekable<Iter<Heading>>,
        parent_level: Option<HeadingLevel>,
        max_end: usize,
    ) -> Vec<HeadingEntry> {
        let mut result: Vec<HeadingEntry> = vec![];

        while let Some(next_heading) = headings.peek() {
            if parent_level.is_some_and(|parent_level| next_heading.level <= parent_level) {
                break;
            }

            if let Some(heading) = headings.next() {
                let next_heading = headings.peek();
                let range_start = heading.index;
                let range_end = next_heading
                    .map(|next_heading| next_heading.index)
                    .unwrap_or(max_end);

                let children = match next_heading {
                    Some(next_heading) if next_heading.level > heading.level => {
                        build_outline_tree_rec(headings, Some(heading.level), max_end)
                    }
                    _ => vec![],
                };

                result.push(HeadingEntry {
                    range: range_start..range_end,
                    level: heading.level,
                    content: heading.content.clone(),
                    children,
                });
            }
        }

        result
    }

    build_outline_tree_rec(&mut headings.iter().peekable(), None, max_end)
}

trait NodesAsHeadings {
    fn to_headings(&self) -> Vec<Heading>;
}

impl NodesAsHeadings for &[Node] {
    fn to_headings(&self) -> Vec<Heading> {
        self.iter()
            .enumerate()
            .filter_map(|(index, node)| {
                if let Node::Heading { level, text, .. } = &node {
                    Some(Heading {
                        index,
                        level: *level,
                        content: text.to_string(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

trait HeadingsAsItems {
    fn to_items(&self, max_end: usize) -> Vec<Item>;
}

impl HeadingsAsItems for Vec<Heading> {
    fn to_items(&self, max_end: usize) -> Vec<Item> {
        build_outline_tree(self, max_end)
            .into_iter()
            .map(Item::from)
            .collect()
    }
}

impl OutlineState {
    pub fn new(nodes: &[Node], index: usize, open: bool) -> Self {
        let headings = nodes.to_headings();

        let mut state = OutlineState {
            open,
            selected_item_index: None,
            items: headings.to_items(nodes.len()),
            list_state: ListState::default(),
            ..Default::default()
        };
        state.select_at(index);
        state.expand_all();
        state
    }

    pub fn set_nodes(&mut self, nodes: &[Node]) {
        let headings = nodes.to_headings();
        self.items = headings.to_items(nodes.len());
        self.expand_all();
    }

    pub fn selected(&self) -> Option<Item> {
        if let Some(selected) = self.list_state.selected() {
            self.items.flatten().get(selected).cloned()
        } else {
            None
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    fn toggle_item_in_tree(item: &Item, target_range: &Range<usize>, should_toggle: bool) -> Item {
        let item = item.clone();

        match item {
            Item::HeadingEntry {
                range: heading_range,
                content,
                expanded,
                children,
            } => {
                let expanded = if heading_range == *target_range && should_toggle {
                    !expanded
                } else {
                    expanded
                };

                Item::HeadingEntry {
                    range: heading_range.clone(),
                    content,
                    expanded,
                    children: children
                        .iter()
                        .map(|item| Self::toggle_item_in_tree(item, target_range, should_toggle))
                        .collect(),
                }
            }
            _ => item,
        }
    }

    pub fn toggle_item(&mut self) {
        let index = self.list_state.selected().unwrap_or_default();

        let items = self.items.flatten();
        let selected_item = items.get(index);

        if let Some(Item::HeadingEntry { range, .. }) = selected_item {
            let target_range = range.clone();

            self.items = self
                .items
                .iter()
                .map(|item| Self::toggle_item_in_tree(item, &target_range, true))
                .collect();
        };
    }

    pub fn select_at(&mut self, index: usize) {
        let (selected_item_index, _) = self.items.find_item(index).unzip();
        self.selected_item_index = selected_item_index;
        self.list_state.select(selected_item_index);
    }

    fn expanded_to_all_items(items: &[Item], expanded: bool) -> Vec<Item> {
        items
            .iter()
            .map(|item| match item {
                Item::HeadingEntry {
                    range,
                    content,
                    children,
                    ..
                } => Item::HeadingEntry {
                    range: range.clone(),
                    content: content.clone(),
                    children: Self::expanded_to_all_items(children, expanded),
                    expanded,
                },
                heading => heading.clone(),
            })
            .collect()
    }

    fn get_visible_item_count(&self) -> usize {
        fn item_count(items: &[Item]) -> usize {
            items.iter().fold(0, |acc, item| {
                let next = acc + 1;
                match item {
                    Item::HeadingEntry {
                        expanded: true,
                        children,
                        ..
                    } => next + item_count(children),
                    _ => next,
                }
            })
        }

        item_count(&self.items)
    }

    pub fn expand_all(&mut self) {
        self.items = Self::expanded_to_all_items(self.items.as_slice(), true);
    }

    pub fn collapse_all(&mut self) {
        self.items = Self::expanded_to_all_items(self.items.as_slice(), false);
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn next(&mut self, amount: usize) {
        let index = self
            .list_state
            .selected()
            .map(|i| (i + amount).min(self.get_visible_item_count().saturating_sub(1)))
            .unwrap_or_default();
        self.list_state.select(Some(index));
    }

    pub fn previous(&mut self, amount: usize) {
        let index = self.list_state.selected().map(|i| i.saturating_sub(amount));
        self.list_state.select(index);
    }
}
