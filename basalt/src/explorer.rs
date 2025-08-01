mod item;
mod state;

pub use item::Item;
pub use state::ExplorerState;
pub use state::Sort;

use std::{marker::PhantomData, path::PathBuf};

use basalt_core::obsidian::Note;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, StatefulWidget},
};

const SORT_SYMBOL_ASC: &str = "↑𝌆";
const SORT_SYMBOL_DESC: &str = "↓𝌆";

#[derive(Default)]
pub struct Explorer<'a> {
    _lifetime: PhantomData<&'a ()>,
}

impl Explorer<'_> {
    pub fn new() -> Self {
        Self {
            _lifetime: PhantomData::<&()>,
        }
    }

    fn list_item<'a>(
        selected_path: Option<PathBuf>,
        is_open: bool,
    ) -> impl Fn(&'a (Item, usize)) -> ListItem<'a> {
        move |(item, depth)| {
            let indentation = if *depth > 0 {
                Span::raw("│ ".repeat(*depth)).black()
            } else {
                Span::raw("  ".repeat(*depth)).black()
            };
            match item {
                Item::File(Note { path, name }) => {
                    let is_selected = selected_path
                        .as_ref()
                        .is_some_and(|selected| selected == path);
                    ListItem::new(Line::from(match (is_open, is_selected) {
                        (true, true) => [indentation, "◆ ".into(), name.into()].to_vec(),
                        (true, false) => [indentation, "  ".into(), name.into()].to_vec(),
                        (false, true) => ["◆".into()].to_vec(),
                        (false, false) => ["◦".dark_gray()].to_vec(),
                    }))
                }
                Item::Directory { expanded, name, .. } => {
                    ListItem::new(Line::from(match (is_open, expanded) {
                        (true, true) => [indentation, "▾ ".dark_gray(), name.into()].to_vec(),
                        (true, false) => [indentation, "▸ ".dark_gray(), name.into()].to_vec(),
                        (false, true) => ["▪".dark_gray()].to_vec(),
                        (false, false) => ["▫".dark_gray()].to_vec(),
                    }))
                }
            }
        }
    }
}

impl<'a> StatefulWidget for Explorer<'a> {
    type State = ExplorerState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::bordered()
            .border_type(if state.active {
                BorderType::Thick
            } else {
                BorderType::Rounded
            })
            .title_style(Style::default().italic().bold());

        let Rect { height, .. } = block.inner(area);
        state.update_offset_mut(height.into());

        let sort_symbol = match state.sort {
            Sort::Asc => SORT_SYMBOL_ASC,
            Sort::Desc => SORT_SYMBOL_DESC,
        };

        let items: Vec<ListItem> = state
            .flat_items
            .iter()
            .map(Explorer::list_item(state.selected_path(), state.is_open()))
            .collect();

        if state.open {
            List::new(items)
                .block(
                    block.title(format!(" {} ", state.title)).title(
                        Line::from([" ".into(), sort_symbol.into(), " ◀ ".into()].to_vec())
                            .alignment(Alignment::Right),
                    ),
                )
                .highlight_style(Style::new().reversed().dark_gray())
                .highlight_symbol(" ")
                .render(area, buf, &mut state.list_state);
        } else {
            let layout = Layout::horizontal([Constraint::Length(5)]).split(area);

            List::new(items)
                .block(block.title(" ▶ "))
                .highlight_style(Style::new().reversed().dark_gray())
                .highlight_symbol(" ")
                .render(layout[0], buf, &mut state.list_state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use basalt_core::obsidian::VaultEntry;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_render_entries() {
        let tests = [
            [].to_vec(),
            [
                VaultEntry::File(Note {
                    name: "Test".into(),
                    path: "test.md".into(),
                }),
                VaultEntry::File(Note {
                    name: "Andesite".into(),
                    path: "andesite.md".into(),
                }),
            ]
            .to_vec(),
            [VaultEntry::Directory {
                name: "TestDir".into(),
                path: "test_dir".into(),
                entries: vec![],
            }]
            .to_vec(),
            [VaultEntry::Directory {
                name: "TestDir".into(),
                path: "test_dir".into(),
                entries: vec![
                    VaultEntry::File(Note {
                        name: "Andesite".into(),
                        path: "test_dir/andesite.md".into(),
                    }),
                    VaultEntry::Directory {
                        name: "Notes".into(),
                        path: "test_dir/notes".into(),
                        entries: vec![VaultEntry::File(Note {
                            name: "Pathing".into(),
                            path: "test_dir/notes/pathing.md".into(),
                        })],
                    },
                    VaultEntry::Directory {
                        name: "Amber Specs".into(),
                        path: "test_dir/amber_specs".into(),
                        entries: vec![VaultEntry::File(Note {
                            name: "Spec_01".into(),
                            path: "test_dir/amber_specs/spec_01.md".into(),
                        })],
                    },
                ],
            }]
            .to_vec(),
        ];

        let mut terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();

        tests.into_iter().for_each(|items| {
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    Explorer::default().render(
                        frame.area(),
                        frame.buffer_mut(),
                        &mut ExplorerState::new("Test", items).select().sort(),
                    )
                })
                .unwrap();
            assert_snapshot!(terminal.backend());
        });
    }
}
