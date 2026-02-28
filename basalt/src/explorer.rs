mod item;
mod state;

use basalt_core::obsidian::directory::Directory;
pub use item::Item;
use ratatui::layout::Position;
use ratatui::layout::Size;
use ratatui::widgets::Borders;
pub use state::ExplorerState;
pub use state::Sort;
pub use state::Visibility;

use std::{marker::PhantomData, path::PathBuf};

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, StatefulWidget},
};

use crate::app::{
    calc_scroll_amount, ActivePane, Message as AppMessage, ScrollAmount, SelectedNote,
};
use crate::input;
use crate::input::InputModalConfig;
use crate::outline;

const SORT_SYMBOL_ASC: &str = "↑≡";
const SORT_SYMBOL_DESC: &str = "↓≡";

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Up,
    Down,
    Open,
    Sort,
    Toggle,
    ToggleOutline,
    ToggleInputRename,
    HidePane,
    ExpandPane,
    SwitchPaneNext,
    SwitchPanePrevious,
    ScrollUp(ScrollAmount),
    ScrollDown(ScrollAmount),
    ScrollToTop,
    ScrollToBottom,
}

pub fn update<'a>(
    message: &Message,
    screen_size: Size,
    state: &mut ExplorerState,
) -> Option<AppMessage<'a>> {
    match message {
        Message::Up => state.previous(1),
        Message::Down => state.next(1),
        Message::Sort => state.sort(),
        Message::Toggle => state.toggle(),
        Message::HidePane => state.hide_pane(),
        Message::ExpandPane => state.expand_pane(),
        Message::SwitchPaneNext => {
            state.set_active(false);
            return Some(AppMessage::SetActivePane(ActivePane::NoteEditor));
        }
        Message::SwitchPanePrevious => {
            state.set_active(false);
            return Some(AppMessage::SetActivePane(ActivePane::Outline));
        }
        Message::ScrollUp(scroll_amount) => {
            state.previous(calc_scroll_amount(scroll_amount, screen_size.height.into()));
        }
        Message::ScrollDown(scroll_amount) => {
            state.next(calc_scroll_amount(scroll_amount, screen_size.height.into()));
        }
        Message::ScrollToTop => {
            state.previous(usize::MAX);
        }
        Message::ScrollToBottom => {
            state.next(usize::MAX);
        }
        Message::ToggleOutline => {
            return Some(AppMessage::Outline(outline::Message::Toggle));
        }
        Message::ToggleInputRename => {
            if let Some(current_item) = state.current_item() {
                let selected_index = state.list_state.selected().unwrap_or(0);
                let (label, input, callback) = match current_item {
                    Item::File(note) => {
                        let input = note.name();
                        ("Rename", input, input::Callback::RenameNote(note.clone()))
                    }
                    Item::Directory { name, path, .. } => (
                        "Rename Directory",
                        name.as_str(),
                        input::Callback::RenameDir(Directory::new(name, path)),
                    ),
                };
                return Some(AppMessage::Input(input::Message::Open(InputModalConfig {
                    // Offset of 2 is used to move the area two rows down so that the original row is visible.
                    position: Position::from((
                        2,
                        (selected_index + 2).saturating_sub(state.list_state.offset()) as u16,
                    )),
                    label: label.to_string(),
                    initial_input: input.to_string(),
                    callback,
                })));
            }
        }
        Message::Open => {
            state.select();
            let note = state.selected_note.as_ref()?;
            return Some(AppMessage::SelectNote(SelectedNote::from(note)));
        }
    };

    None
}

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
                Item::File(note) => {
                    let name = note.name();
                    let path = note.path();

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
    type State = ExplorerState;

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

        if state.is_open() {
            List::new(items)
                .block(
                    block
                        .title(format!(
                            "{} {} ",
                            if state.visibility == Visibility::FullWidth {
                                " ⟹ "
                            } else {
                                ""
                            },
                            state.title
                        ))
                        .title(
                            Line::from(vec![" ".into(), sort_symbol.into(), " ◀ ".into()])
                                .alignment(Alignment::Right),
                        ),
                )
                .highlight_style(Style::new().reversed().dark_gray())
                .highlight_symbol(" ")
                .render(area, buf, &mut state.list_state);
        } else {
            let layout = Layout::horizontal([Constraint::Length(5)]).split(area);

            List::new(items)
                .block(
                    block
                        .title(" ▶ ")
                        .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM),
                )
                .highlight_style(Style::new().reversed().dark_gray())
                .highlight_symbol(" ")
                .render(layout[0], buf, &mut state.list_state);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use basalt_core::obsidian::{Note, VaultEntry};
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_toggle_input_rename_position_accounts_for_scroll_offset() {
        let items: Vec<VaultEntry> = (0..30)
            .map(|i| {
                VaultEntry::File(Note::new_unchecked(
                    &format!("Note_{i}"),
                    Path::new(&format!("Note_{i}.md")),
                ))
            })
            .collect();

        let mut state = ExplorerState::new("Test", items);
        state.next(25);

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal
            .draw(|frame| Explorer::default().render(frame.area(), frame.buffer_mut(), &mut state))
            .unwrap();

        let offset = state.list_state.offset();
        assert!(offset > 0, "offset should be non-zero after scrolling");

        let selected = state.list_state.selected().unwrap();
        let result = update(&Message::ToggleInputRename, Size::new(80, 20), &mut state);
        let expected_y = (selected + 2).saturating_sub(offset) as u16;

        match result {
            Some(AppMessage::Input(input::Message::Open(config))) => {
                assert_eq!(config.position, Position::from((2, expected_y)));
            }
            other => panic!("Expected AppMessage::Input(Open(..)), got: {other:?}"),
        }
    }

    #[test]
    fn test_render_entries() {
        let tests = [
            [].to_vec(),
            [
                VaultEntry::File(Note::new_unchecked("Test", Path::new("Test.md"))),
                VaultEntry::File(Note::new_unchecked("Andesite", Path::new("Andesite.md"))),
            ]
            .to_vec(),
            [VaultEntry::Directory {
                name: "TestDir".into(),
                path: "TestDir".into(),
                entries: vec![],
            }]
            .to_vec(),
            [VaultEntry::Directory {
                name: "TestDir".into(),
                path: "TestDir".into(),
                entries: vec![
                    VaultEntry::File(Note::new_unchecked("Andesite", Path::new("Andesite.md"))),
                    VaultEntry::Directory {
                        name: "Notes".into(),
                        path: "TestDir/Notes".into(),
                        entries: vec![VaultEntry::File(Note::new_unchecked(
                            "Pathing",
                            Path::new("TestDir/Notes/Pathing.md"),
                        ))],
                    },
                    VaultEntry::Directory {
                        name: "Amber Specs".into(),
                        path: "TestDir/Amber Specs".into(),
                        entries: vec![VaultEntry::File(Note::new_unchecked(
                            "Spec_01",
                            Path::new("TestDir/Amber Specs/Spec_01.md"),
                        ))],
                    },
                ],
            }]
            .to_vec(),
        ];

        let mut terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();

        tests.into_iter().for_each(|items| {
            _ = terminal.clear();
            let mut state = ExplorerState::new("Test", items);
            state.select();
            state.sort();

            terminal
                .draw(|frame| {
                    Explorer::default().render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            assert_snapshot!(terminal.backend());
        });
    }
}
