use item::{Flatten, Item};
pub use state::OutlineState;

mod item;
mod state;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, StatefulWidget},
};

use crate::{
    app::{ActivePane, Message as AppMessage},
    config::Symbols,
    explorer,
    note_editor::{self, ast::Node},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Up,
    Down,
    Select,
    SelectAt(usize),
    SetNodes(Vec<Node>),
    Expand,
    Toggle,
    ToggleExplorer,
    SwitchPaneNext,
    SwitchPanePrevious,
}

pub fn update<'a>(message: &Message, state: &mut OutlineState) -> Option<AppMessage<'a>> {
    match message {
        Message::Up => state.previous(1),
        Message::Down => state.next(1),
        Message::Expand => state.toggle_item(),
        Message::SelectAt(index) => state.select_at(*index),
        Message::SetNodes(nodes) => state.set_nodes(nodes),

        Message::SwitchPaneNext => {
            state.set_active(false);
            return Some(AppMessage::SetActivePane(ActivePane::Explorer));
        }
        Message::SwitchPanePrevious => {
            state.set_active(false);
            return Some(AppMessage::SetActivePane(ActivePane::NoteEditor));
        }
        Message::Toggle => state.toggle(),
        Message::Select => {
            if let Some(item) = state.selected() {
                // This is a block idx, not a source range offset
                let block_idx = item.get_range().start;
                return Some(AppMessage::NoteEditor(note_editor::Message::JumpToBlock(
                    block_idx,
                )));
            }
        }
        Message::ToggleExplorer => {
            return Some(AppMessage::Explorer(explorer::Message::Toggle));
        }
    };

    None
}

#[derive(Default)]
pub struct Outline;

trait AsListItems {
    fn to_list_items<'a>(&'a self, symbols: &'a Symbols) -> Vec<ListItem<'a>>;
    fn to_collapsed_items<'a>(&'a self, symbols: &'a Symbols) -> Vec<ListItem<'a>>;
}

impl AsListItems for Vec<Item> {
    fn to_collapsed_items<'a>(&'a self, symbols: &'a Symbols) -> Vec<ListItem<'a>> {
        self.flatten()
            .iter()
            .map(|item| match item {
                Item::Heading { .. } => {
                    ListItem::new(Line::from(symbols.outline_heading_dot.as_str()))
                        .dark_gray()
                        .dim()
                }
                Item::HeadingEntry { expanded: true, .. } => {
                    ListItem::new(Line::from(symbols.outline_heading_expanded.as_str()))
                        .red()
                        .dim()
                }
                Item::HeadingEntry {
                    expanded: false, ..
                } => ListItem::new(Line::from(symbols.outline_heading_collapsed.as_str()))
                    .dark_gray()
                    .dim(),
            })
            .collect()
    }

    fn to_list_items<'a>(&'a self, symbols: &'a Symbols) -> Vec<ListItem<'a>> {
        fn list_item<'a>(
            indentation: Span<'a>,
            symbol: Span<'a>,
            content: &'a str,
        ) -> ListItem<'a> {
            ListItem::new(Line::from([indentation, symbol, content.into()].to_vec()))
        }

        fn to_list_items_inner<'a>(
            depth: usize,
            symbols: &'a Symbols,
        ) -> impl Fn(&'a Item) -> Vec<ListItem<'a>> {
            let indentation = if depth > 0 {
                Span::raw(format!("{} ", symbols.outline_indent).repeat(depth)).black()
            } else {
                Span::raw("  ".repeat(depth)).black()
            };
            let expanded_marker = Span::from(format!("{} ", symbols.outline_expanded));
            let collapsed_marker = Span::from(format!("{} ", symbols.outline_collapsed));
            move |item| match item {
                Item::Heading { content, .. } => {
                    vec![list_item(indentation.clone(), "  ".into(), content)]
                }
                Item::HeadingEntry {
                    expanded: true,
                    children,
                    content,
                    ..
                } => {
                    let mut items = vec![list_item(
                        indentation.clone(),
                        expanded_marker.clone(),
                        content,
                    )];
                    items.extend(
                        children
                            .iter()
                            .flat_map(to_list_items_inner(depth + 1, symbols)),
                    );
                    items
                }
                Item::HeadingEntry {
                    expanded: false,
                    content,
                    ..
                } => vec![list_item(
                    indentation.clone(),
                    collapsed_marker.clone(),
                    content,
                )],
            }
        }

        self.iter()
            .flat_map(to_list_items_inner(0, symbols))
            .collect()
    }
}

impl StatefulWidget for Outline {
    type State = OutlineState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::bordered()
            .border_type(if state.active {
                state.symbols.border_active.into()
            } else {
                state.symbols.border_inactive.into()
            })
            .title(if state.is_open() {
                format!(" {} Outline ", state.symbols.pane_open)
            } else {
                format!(" {} ", state.symbols.pane_close)
            })
            .title_alignment(Alignment::Right)
            .padding(Padding::horizontal(1))
            .title_style(Style::default().italic().bold());

        let items = if state.is_open() {
            state.items.to_list_items(&state.symbols)
        } else {
            state.items.to_collapsed_items(&state.symbols)
        };

        List::new(items)
            .block(if state.is_open() {
                block
            } else {
                block.borders(Borders::RIGHT | Borders::TOP | Borders::BOTTOM)
            })
            .highlight_style(Style::default().reversed().dark_gray())
            .highlight_symbol("")
            .render(area, buf, &mut state.list_state);
    }
}

#[cfg(test)]
mod tests {
    use crate::note_editor::parser;

    use super::*;
    use indoc::indoc;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_outline_render() {
        let tests = [
            ("empty", parser::from_str("")),
            ("single_level", parser::from_str("# Heading 1")),
            (
                "only_top_level",
                parser::from_str(indoc! {r#"
                # Heading 1
                # Heading 2
                # Heading 3
                # Heading 4
                # Heading 5
                # Heading 6
            "#}),
            ),
            (
                "only_deep_level",
                parser::from_str(indoc! {r#"
                ###### Heading 1
                ##### Heading 2
                ###### Heading 2.1
                ###### Heading 2.2
                ##### Heading 3
                ##### Heading 4
                ###### Heading 4.1
                ##### Heading 5
            "#}),
            ),
            (
                "sequential_all_levels",
                parser::from_str(indoc! {r#"
                # Heading 1
                ## Heading 2
                ### Heading 3
                #### Heading 4
                ##### Heading 5
                ###### Heading 6
            "#}),
            ),
            (
                "complex_nested_structure",
                parser::from_str(indoc! {r#"
                ## Heading 1
                ## Heading 2
                ### Heading 2.1
                #### Heading 2.1.1
                ### Heading 2.2
                #### Heading 2.2.1
                ## Heading 3
                ###### Heading 3.1.1.1.1.1
            "#}),
            ),
            (
                "irregular_nesting_with_skips",
                parser::from_str(indoc! {r#"
                # Heading 1
                ## Heading 2
                ## Heading 2.1
                #### Heading 2.1.1
                #### Heading 2.1.2
                ## Heading 2.2
                ### Heading 3
            "#}),
            ),
            (
                "level_skipping",
                parser::from_str(indoc! {r#"
                # Level 1
                ### Level 3 (skipped 2)
                ##### Level 5 (skipped 4)
                ## Level 2 (back to 2)
                ###### Level 6 (jump to 6)
            "#}),
            ),
            (
                "reverse_hierarchy",
                parser::from_str(indoc! {r#"
                ###### Level 6
                ##### Level 5
                #### Level 4
                ### Level 3
                ## Level 2
                # Level 1
            "#}),
            ),
            (
                "multiple_root_levels",
                parser::from_str(indoc! {r#"
                # Root 1
                ## Child 1.1
                ### Child 1.1.1

                ## Root 2 (different level)
                #### Child 2.1 (skipped level 3)

                ### Root 3 (different level)
                ###### Child 3.1 (deep skip)
            "#}),
            ),
            (
                "duplicate_headings",
                parser::from_str(indoc! {r#"
                # Duplicate
                ## Child
                # Duplicate
                ## Different Child
                # Duplicate
            "#}),
            ),
            (
                "mixed_with_content",
                parser::from_str(indoc! {r#"
                # Chapter 1
                Some paragraph content here.

                ## Section 1.1
                More content.

                - List item
                - Another item

                ### Subsection 1.1.1
                Final content.
            "#}),
            ),
            (
                "boundary_conditions_systematic",
                parser::from_str(indoc! {r#"
                # A
                ## B
                ### C
                #### D
                ##### E
                ###### F
                ##### E2
                #### D2
                ### C2
                ## B2
                # A2
            "#}),
            ),
        ];

        let mut terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();

        tests.into_iter().for_each(|(name, nodes)| {
            _ = terminal.clear();
            let mut state = OutlineState::new(&nodes, 0, true, &Symbols::unicode());
            state.expand_all();
            terminal
                .draw(|frame| Outline.render(frame.area(), frame.buffer_mut(), &mut state))
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }
}
