use std::marker::PhantomData;

use ratatui::{
    buffer::Buffer,
    layout::{Offset, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        self, Block, BorderType, Clear, Padding, Paragraph, ScrollbarOrientation, StatefulWidget,
        Widget,
    },
};

use crate::{
    note_editor::{ast, rich_text::RichText},
    stylized_text::{stylize, FontStyle},
};

use super::state::View;

use super::state::EditorState;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Editor<'text_buffer>(PhantomData<&'text_buffer ()>);

impl Editor<'_> {
    fn task<'a>(kind: ast::TaskKind, content: Vec<Span<'a>>, prefix: Span<'a>) -> Line<'a> {
        // TODO: Create an utility to insert n amount of spans with a symbol
        let space = Span::from(" ");

        match kind {
            ast::TaskKind::Unchecked => Line::from(
                [prefix, "□".dark_gray(), space]
                    .into_iter()
                    .chain(content)
                    .collect::<Vec<_>>(),
            ),
            ast::TaskKind::Checked => {
                let crossed_out_content = content
                    .into_iter()
                    .map(|span| span.dark_gray().add_modifier(Modifier::CROSSED_OUT));

                Line::from(
                    [prefix, "■".magenta(), space]
                        .into_iter()
                        .chain(crossed_out_content)
                        .collect::<Vec<_>>(),
                )
            }
            ast::TaskKind::LooselyChecked => Line::from(
                [prefix, "■".magenta(), space]
                    .into_iter()
                    .chain(content)
                    .collect::<Vec<_>>(),
            ),
        }
    }

    fn item<'a>(kind: ast::ListKind, content: Vec<Span<'a>>, prefix: Span<'a>) -> Line<'a> {
        match kind {
            ast::ListKind::Ordered(num) => Line::from(
                [prefix, num.to_string().dark_gray(), ". ".into()]
                    .into_iter()
                    .chain(content)
                    .collect::<Vec<_>>(),
            ),
            ast::ListKind::Unordered => Line::from(
                [prefix, "- ".dark_gray()]
                    .into_iter()
                    .chain(content)
                    .collect::<Vec<_>>(),
            ),
        }
    }

    fn text_to_spans<'a>(text: RichText) -> Vec<Span<'a>> {
        text.into_iter()
            // TODO: Styling
            .map(|text| Span::from(text.content))
            .collect()
    }

    fn code_block<'a>(text: &RichText, width: usize) -> Vec<Line<'a>> {
        text.iter()
            .flat_map(|text| {
                text.content
                    .clone()
                    .split("\n")
                    .map(|line| {
                        format!(
                            " {} {}",
                            line,
                            // We subtract two to take the whitespace into account, which are
                            // added in the format string.
                            (line.chars().count()..width.saturating_sub(2))
                                .map(|_| " ")
                                .collect::<String>()
                        )
                    })
                    .collect::<Vec<String>>()
            })
            .map(|text| Line::from(text).bg(Color::Black))
            .collect()
    }

    fn wrap_with_prefix(text: String, width: usize, prefix: Span) -> Vec<Line> {
        let options =
            textwrap::Options::new(width.saturating_sub(prefix.width())).break_words(false);

        textwrap::wrap(&text, &options)
            .into_iter()
            .map(|wrapped_line| {
                Line::from([prefix.clone(), Span::from(wrapped_line.to_string())].to_vec())
            })
            .collect()
    }

    fn heading<'a>(level: ast::HeadingLevel, text: String, width: usize) -> Vec<Line<'a>> {
        match level {
            ast::HeadingLevel::H1 => [
                Line::from(text.to_uppercase()).bold(),
                (0..width).map(|_| "═").collect::<String>().into(),
                Line::default(),
            ]
            .to_vec(),
            ast::HeadingLevel::H2 => [
                Line::from(text).bold().yellow(),
                Line::from((0..width).map(|_| "─").collect::<String>()).yellow(),
            ]
            .to_vec(),
            ast::HeadingLevel::H3 => [
                Line::from(["⬤  ".into(), text.bold()].to_vec()).cyan(),
                Line::default(),
            ]
            .to_vec(),
            ast::HeadingLevel::H4 => [
                Line::from(["● ".into(), text.bold()].to_vec()).magenta(),
                Line::default(),
            ]
            .to_vec(),
            ast::HeadingLevel::H5 => [
                Line::from(["◆ ".into(), stylize(&text, FontStyle::Script).into()].to_vec()),
                Line::default(),
            ]
            .to_vec(),
            ast::HeadingLevel::H6 => [
                Line::from(["✺ ".into(), stylize(&text, FontStyle::Script).into()].to_vec()),
                Line::default(),
            ]
            .to_vec(),
        }
    }

    fn render_markdown<'a>(node: &ast::Node, area: Rect, prefix: Span<'a>) -> Vec<Line<'a>> {
        match node {
            ast::Node::Paragraph { text, .. } => {
                Editor::wrap_with_prefix(text.to_string(), area.width.into(), prefix.clone())
                    .into_iter()
                    .chain(if prefix.to_string().is_empty() {
                        [Line::default()].to_vec()
                    } else {
                        [].to_vec()
                    })
                    .collect::<Vec<_>>()
            }
            ast::Node::Heading { level, text, .. } => {
                Editor::heading(*level, text.to_string(), area.width.into())
            }
            ast::Node::Item { nodes, .. } => [Editor::item(
                ast::ListKind::Unordered,
                Editor::text_to_spans(
                    nodes
                        .first()
                        .map(|node| match node {
                            ast::Node::Paragraph { text, .. } => text.clone(),
                            _ => RichText::empty(),
                        })
                        .unwrap_or(RichText::empty()),
                ),
                prefix,
            )]
            .to_vec(),
            ast::Node::Task { kind, nodes, .. } => [Editor::task(
                kind.clone(),
                Editor::text_to_spans(
                    nodes
                        .first()
                        .map(|node| match node {
                            ast::Node::Paragraph { text, .. } => text.clone(),
                            _ => RichText::empty(),
                        })
                        .unwrap_or(RichText::empty()),
                ),
                prefix,
            )]
            .to_vec(),
            // TODO: Add lang support and syntax highlighting
            ast::Node::CodeBlock { text, .. } => {
                [Line::from((0..area.width).map(|_| " ").collect::<String>()).bg(Color::Black)]
                    .into_iter()
                    .chain(Editor::code_block(text, area.width.into()))
                    .chain([Line::default()])
                    .collect::<Vec<_>>()
            }
            ast::Node::List { nodes, kind, .. } => nodes
                .iter()
                .enumerate()
                .flat_map(|(i, child)| match child {
                    ast::Node::Task { kind, nodes, .. } => [Editor::task(
                        kind.clone(),
                        Editor::text_to_spans(
                            nodes
                                .first()
                                .map(|node| match node {
                                    ast::Node::Paragraph { text, .. } => text.clone(),
                                    _ => RichText::empty(),
                                })
                                .unwrap_or(RichText::empty()),
                        ),
                        prefix.clone(),
                    )]
                    .to_vec(),
                    ast::Node::Item { nodes, .. } => {
                        let item = match kind {
                            ast::ListKind::Ordered(start) => Editor::item(
                                ast::ListKind::Ordered(start + i as u64),
                                Editor::text_to_spans(
                                    nodes
                                        .first()
                                        .map(|node| match node {
                                            ast::Node::Paragraph { text, .. } => text.clone(),
                                            _ => RichText::empty(),
                                        })
                                        .unwrap_or(RichText::empty()),
                                ),
                                prefix.clone(),
                            ),
                            _ => Editor::item(
                                ast::ListKind::Unordered,
                                Editor::text_to_spans(
                                    nodes
                                        .first()
                                        .map(|node| match node {
                                            ast::Node::Paragraph { text, .. } => text.clone(),
                                            _ => RichText::empty(),
                                        })
                                        .unwrap_or(RichText::empty()),
                                ),
                                prefix.clone(),
                            ),
                        };

                        [item].to_vec()
                    }
                    _ => Editor::render_markdown(child, area, Span::from(format!("  {prefix}"))),
                })
                .chain(if prefix.to_string().is_empty() {
                    [Line::default()].to_vec()
                } else {
                    [].to_vec()
                })
                .collect::<Vec<Line<'a>>>(),

            // TODO: Support callout block quote types
            ast::Node::BlockQuote { nodes, .. } => nodes
                .iter()
                .map(|child| {
                    // We need this to be a block of lines to make sure we enumarate and add
                    // prefixed line breaks correctly.
                    [Editor::render_markdown(
                        child,
                        area,
                        Span::from(prefix.to_string() + "┃ ").magenta(),
                    )]
                    .to_vec()
                })
                .enumerate()
                .flat_map(|(i, mut line_blocks)| {
                    if i != 0 && i != nodes.len() {
                        line_blocks.insert(
                            0,
                            [Line::from(prefix.to_string() + "┃ ").magenta()].to_vec(),
                        );
                    }
                    line_blocks.into_iter().flatten().collect::<Vec<_>>()
                })
                .chain(if prefix.to_string().is_empty() {
                    [Line::default()].to_vec()
                } else {
                    [].to_vec()
                })
                .collect::<Vec<Line<'a>>>(),
        }
    }
}

impl<'text_buffer> StatefulWidget for Editor<'text_buffer> {
    type State = EditorState<'text_buffer>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mode_color = match state.view {
            View::Edit(..) => Color::Green,
            View::Read => Color::Red,
        };

        let block = Block::bordered()
            .border_type(if state.active() {
                BorderType::Thick
            } else {
                BorderType::Rounded
            })
            .title_bottom(
                [
                    format!(" {}", state.view).fg(mode_color).bold().italic(),
                    if state.modified {
                        "* ".bold().italic()
                    } else {
                        " ".into()
                    },
                ]
                .to_vec(),
            )
            .padding(Padding::horizontal(1));

        let inner_area = block.inner(area);

        let nodes = state.nodes();

        let rendered_nodes: Vec<_> = nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                // TODO: Figure out how to wrap the text while editing / viewing the markdown
                // blocks.
                //
                // The following code is not good, but might act as something that can be
                // considered.
                //
                // let (row, col) = state.text_buffer().cursor();
                //
                // let mut ta = TextArea::from(
                //     wrap_text(state.text_buffer().raw(), inner_area.width as usize).lines(),
                // );
                //
                // let offset_row = col as u16 / inner_area.width;
                //
                // ta.move_cursor(tui_textarea::CursorMove::Jump(
                //     row as u16 + offset_row,
                //     if offset_row > 0 {
                //         (col as u16).saturating_sub(inner_area.width * offset_row) as u16
                //     } else {
                //         col as u16
                //     },
                // ));

                match (i == state.current_row, &state.view) {
                    (true, View::Read) => {
                        let (row, _) = state.text_buffer().cursor();
                        Editor::render_markdown(node, inner_area, Span::default())
                            .into_iter()
                            .enumerate()
                            .map(|(i, line)| if i == row { line.underlined() } else { line })
                            .collect()
                    }
                    (true, _) => {
                        let expected_line_count =
                            Editor::render_markdown(node, inner_area, Span::default()).len();

                        let mut buffer_lines: Vec<Line> = state
                            .text_buffer()
                            .lines()
                            .iter()
                            .map(|line| Line::from(line.clone()))
                            .collect();

                        if buffer_lines.len() < expected_line_count {
                            buffer_lines.resize(expected_line_count.max(1), Line::default());
                        }

                        buffer_lines
                    }
                    (false, _) => Editor::render_markdown(node, inner_area, Span::default()),
                }
            })
            .collect();

        let offset_row = if !rendered_nodes.is_empty() {
            rendered_nodes[..state.current_row]
                .iter()
                .map(|lines| lines.len())
                .sum::<usize>()
        } else {
            0
        };

        let current_node_height = rendered_nodes
            .get(state.current_row)
            .map_or(0, |lines| lines.len() as u16);

        fn calculate_clipped_rows(offset: i16, pos_y: u16, height: u16, max: u16) -> u16 {
            if offset < 0 {
                height.saturating_sub(height.saturating_sub(offset.unsigned_abs()))
            } else {
                (pos_y + height).saturating_sub(max)
            }
        }

        let get_heading_lines = || match !state.file_name.is_empty() {
            true => vec![
                Line::from(stylize(&state.file_name, FontStyle::BlackBoardBold)),
                Line::from((0..inner_area.width).map(|_| "═").collect::<String>()),
                Line::default(),
            ],
            false => vec![],
        };

        let heading_lines = get_heading_lines();
        let heading_lines_len = heading_lines.len();

        let scrollbar = state.scrollbar();

        // We take the borders into consideration, thus we add 1, otherwise the calculated
        // rect would be rendered over the block border.
        let unsigned_clamped_vertical_offset = (offset_row + heading_lines_len + 1)
            .saturating_sub(scrollbar.position)
            .max(1) as u16;

        let vertical_offset = offset_row as i16 - scrollbar.position as i16;

        let max_height = inner_area.bottom();

        // Amount of rows that get clipped
        let clipped_rows = calculate_clipped_rows(
            vertical_offset,
            unsigned_clamped_vertical_offset,
            current_node_height,
            max_height,
        );

        let rect = Rect::new(
            0,
            0,
            inner_area.width,
            current_node_height.saturating_sub(clipped_rows),
        )
        .offset(Offset {
            x: inner_area.x as i32,
            y: unsigned_clamped_vertical_offset as i32,
        })
        .clamp(inner_area);

        let content_lines = rendered_nodes.into_iter().flatten().collect::<Vec<_>>();
        let content_lines_len = content_lines.len();
        let mut scroll_state = scrollbar.state.content_length(content_lines_len);

        let lines = [heading_lines, content_lines].concat();
        let lines_len = lines.len();

        let root_node = Paragraph::new(lines)
            .block(block)
            .scroll((scrollbar.position as u16, 0));

        Widget::render(root_node, area, buf);

        // TODO: Investigate why crash happens when complete node is rendered
        if rect.top() < max_height && state.view != View::Read {
            // Nothing is visible, so we exit early
            if (vertical_offset < 0 && clipped_rows == 0) || state.view == View::Read {
                return;
            }

            let buffer = state.text_buffer_as_mut();
            let textarea = buffer.textarea_as_mut();

            if vertical_offset > 0 && clipped_rows != 0 {
                let (row, col) = textarea.cursor();
                let fixed_scroll = current_node_height.saturating_sub(clipped_rows);

                if (row as u16 + 1) > fixed_scroll {
                    textarea.set_cursor_style(Style::default());
                    textarea.set_cursor_line_style(Style::default());
                    textarea.move_cursor(tui_textarea::CursorMove::Jump(
                        fixed_scroll.saturating_sub(1),
                        col as u16,
                    ));
                }
            } else if vertical_offset < 0 && clipped_rows != 0 {
                let (row, col) = textarea.cursor();
                let row = row as u16;

                textarea.scroll((clipped_rows as i16, 0));

                if row < clipped_rows && textarea.lines().len() > 1 {
                    textarea.move_cursor(tui_textarea::CursorMove::Jump(clipped_rows, col as u16));
                    textarea.set_cursor_style(Style::default());
                    textarea.set_cursor_line_style(Style::default());
                } else {
                    textarea.move_cursor(tui_textarea::CursorMove::Jump(row, col as u16));
                }
            }

            Clear.render(rect, buf);
            textarea.render(rect, buf);
        }

        if lines_len as u16 > inner_area.height && inner_area.width > 0 {
            StatefulWidget::render(
                widgets::Scrollbar::new(ScrollbarOrientation::VerticalRight),
                area,
                buf,
                &mut scroll_state,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::note_editor::state::EditMode;

    use super::*;
    use indoc::indoc;
    use insta::assert_snapshot;
    use ratatui::{
        backend::TestBackend,
        crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
        Terminal,
    };

    #[test]
    fn test_rendered_markdown_view() {
        let tests = [
            indoc! { r#"## Headings

            # This is a heading 1

            ## This is a heading 2

            ### This is a heading 3

            #### This is a heading 4

            ##### This is a heading 5

            ###### This is a heading 6
            "#},
            indoc! { r#"## Quotes

            You can quote text by adding a > symbols before the text.

            > Human beings face ever more complex and urgent problems, and their effectiveness in dealing with these problems is a matter that is critical to the stability and continued progress of society.
            >
            > - Doug Engelbart, 1961
            "#},
            indoc! { r#"## Callout Blocks

            > [!tip]
            >
            >You can turn your quote into a [callout](https://help.obsidian.md/Editing+and+formatting/Callouts) by adding `[!info]` as the first line in a quote.
            "#},
            indoc! { r#"## Deep Quotes

            You can have deeper levels of quotes by adding a > symbols before the text inside the block quote.

            > Regular thoughts
            >
            > > Deeper thoughts
            > >
            > > > Very deep thoughts
            > > >
            > > > - Someone on the internet 1996
            >
            > Back to regular thoughts
            "#},
            indoc! { r#"## Lists

            You can create an unordered list by adding a `-`, `*`, or `+` before the text.

            - First list item
            - Second list item
            - Third list item

            To create an ordered list, start each line with a number followed by a `.` symbol.

            1. First list item
            2. Second list item
            3. Third list item
            "#},
            indoc! { r#"## Indented Lists

            Lists can be indented

            - First list item
              - Second list item
                - Third list item

            "#},
            indoc! { r#"## Task lists

            To create a task list, start each list item with a hyphen and space followed by `[ ]`.

            - [x] This is a completed task.
            - [ ] This is an incomplete task.

            >You can use any character inside the brackets to mark it as complete.

            - [x] Oats
            - [?] Flour
            - [d] Apples
            "#},
            indoc! { r#"## Code blocks

            To format a block of code, surround the code with triple backticks.

            ```
            cd ~/Desktop
            ```

            You can also create a code block by indenting the text using `Tab` or 4 blank spaces.

                cd ~/Desktop
            "#},
            indoc! { r#"## Code blocks

            You can add syntax highlighting to a code block, by adding a language code after the first set of backticks.

            ```js
            function fancyAlert(arg) {
              if(arg) {
                $.facebox({div:'#foo'})
              }
            }
            ```
            "#},
        ];

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();

        tests.iter().for_each(|text| {
            _ = terminal.clear();
            let mut state = EditorState::default();
            state.set_content(text);
            terminal
                .draw(|frame| {
                    Editor::default().render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            assert_snapshot!(terminal.backend());
        });
    }

    #[test]
    fn test_rendered_editor_states() {
        let content = indoc! { r#"## Deep Quotes

            You can have deeper levels of quotes by adding a > symbols before the text inside the block quote.

            > Regular thoughts
            >
            > > Deeper thoughts
            > >
            > > > Very deep thoughts
            > > >
            > > > - Someone on the internet 1996
            >
            > Back to regular thoughts
            "#};

        let tests = [
            ("empty_default_state", EditorState::default()),
            ("default_content", {
                let mut state = EditorState::default();
                state.set_content(content);
                state
            }),
            ("read_mode_with_content", {
                let mut state = EditorState::default();
                state.set_content(content);
                state.set_view(View::Read);
                state
            }),
            ("edit_mode_with_content", {
                let mut state = EditorState::default();
                state.set_content(content);
                state.set_view(View::Edit(EditMode::Source));
                state
            }),
            ("edit_mode_with_content_and_simple_change", {
                let mut state = EditorState::default();
                state.set_content(content);
                state.set_view(View::Edit(EditMode::Source));
                state.edit(KeyEvent::new(KeyCode::Char('#'), KeyModifiers::empty()).into());
                state.exit_insert();
                state.set_view(View::Read);
                state
            }),
            ("edit_mode_with_arbitrary_cursor_move", {
                let mut state = EditorState::default();
                state.set_content(content);
                state.cursor_move_col(7);
                state.set_view(View::Edit(EditMode::Source));
                state.edit(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('B'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()).into());
                state.exit_insert();
                state.set_view(View::Read);
                state
            }),
            ("edit_mode_with_content_with_complete_word_input_change", {
                let mut state = EditorState::default();
                state.set_content(content);
                state.cursor_down();
                state.set_view(View::Edit(EditMode::Source));
                state.edit(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('B'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()).into());
                state.edit(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()).into());
                state.exit_insert();
                state.set_view(View::Read);
                state
            }),
        ];

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();

        tests.into_iter().for_each(|(name, mut state)| {
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    Editor::default().render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }

    #[test]
    fn test_basic_formatting() {
        let tests = [
            (
                "paragraphs",
                indoc! { r#"## Paragraphs
                To create paragraphs in Markdown, use a **blank line** to separate blocks of text. Each block of text separated by a blank line is treated as a distinct paragraph.

                This is a paragraph.

                This is another paragraph.

                A blank line between lines of text creates separate paragraphs. This is the default behavior in Markdown.
                "#},
            ),
            (
                "headings",
                indoc! { r#"## Headings
                To create a heading, add up to six `#` symbols before your heading text. The number of `#` symbols determines the size of the heading.

                # This is a heading 1
                ## This is a heading 2
                ### This is a heading 3
                #### This is a heading 4
                ##### This is a heading 5
                ###### This is a heading 6
                "#},
            ),
            (
                "lists",
                indoc! { r#"## Lists
                You can create an unordered list by adding a `-`, `*`, or `+` before the text.

                - First list item
                - Second list item
                - Third list item

                To create an ordered list, start each line with a number followed by a `.` or `)` symbol.

                1. First list item
                2. Second list item
                3. Third list item

                1) First list item
                2) Second list item
                3) Third list item
                "#},
            ),
            (
                "lists_line_breaks",
                indoc! { r#"## Lists with line breaks
                You can use line breaks within an ordered list without altering the numbering.

                1. First list item

                2. Second list item
                3. Third list item

                4. Fourth list item
                5. Fifth list item
                6. Sixth list item
                "#},
            ),
            (
                "task_lists",
                indoc! { r#"## Task lists
                To create a task list, start each list item with a hyphen and space followed by `[ ]`.

                - [x] This is a completed task.
                - [ ] This is an incomplete task.

                You can toggle a task in Reading view by selecting the checkbox.

                > [!tip]
                > You can use any character inside the brackets to mark it as complete.
                >
                > - [x] Milk
                > - [?] Eggs
                > - [-] Eggs
                "#},
            ),
            (
                "nesting_lists",
                indoc! { r#"## Nesting lists
                You can nest any type of list—ordered, unordered, or task lists—under any other type of list.

                To create a nested list, indent one or more list items. You can mix list types within a nested structure:

                1. First list item
                   1. Ordered nested list item
                2. Second list item
                   - Unordered nested list item
                "#},
            ),
            (
                "nesting_task_lists",
                indoc! { r#"## Nesting task lists
                Similarly, you can create a nested task list by indenting one or more list items:

                - [ ] Task item 1
                 - [ ] Subtask 1
                - [ ] Task item 2
                 - [ ] Subtask 1
                "#},
            ),
            // TODO: Implement horizontal rule
            // (
            //     "horizontal_rule",
            //     indoc! { r#"## Horizontal rule
            //     You can use three or more stars `***`, hyphens `---`, or underscore `___` on its own line to add a horizontal bar. You can also separate symbols using spaces.
            //
            //     ***
            //     ****
            //     * * *
            //     ---
            //     ----
            //     - - -
            //     ___
            //     ____
            //     _ _ _
            //     "#},
            // ),
            (
                "code_blocks",
                indoc! { r#"## Code blocks
                To format code as a block, enclose it with three backticks or three tildes.

                ```md
                cd ~/Desktop
                ```

                You can also create a code block by indenting the text using `Tab` or 4 blank spaces.

                    cd ~/Desktop

                "#},
            ),
            (
                "code_syntax_highlighting_in_blocks",
                indoc! { r#"## Code syntax highlighting in blocks
                You can add syntax highlighting to a code block, by adding a language code after the first set of backticks.

                ```js
                function fancyAlert(arg) {
                  if(arg) {
                    $.facebox({div:'#foo'})
                  }
                }
                ```
                "#},
            ),
        ];

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();

        tests.into_iter().for_each(|(name, content)| {
            let mut state = EditorState::new("Note name", content, PathBuf::default());
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    Editor::default().render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }
}
