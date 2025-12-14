use std::marker::PhantomData;

use ratatui::{
    buffer::Buffer,
    layout::{Offset, Rect},
    style::{Color, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget,
    },
};

use crate::note_editor::{
    cursor::CursorWidget,
    state::{NoteEditorState, View},
};

#[derive(Default)]
pub struct NoteEditor<'a>(pub PhantomData<&'a ()>);

impl<'a> StatefulWidget for NoteEditor<'a> {
    type State = NoteEditorState<'a>;

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
            // NOTE: Uncomment for debugging
            // .title_top(format!(
            //     "{},{},{}",
            //     state.cursor.virtual_line(),
            //     state.cursor.virtual_column(),
            //     state.cursor.source_offset()
            // ))
            .title_bottom(
                [
                    format!(" {}", state.view).fg(mode_color).bold().italic(),
                    if state.modified() {
                        "* ".bold().italic()
                    } else {
                        " ".into()
                    },
                ]
                .to_vec(),
            )
            .padding(Padding::horizontal(1));

        let inner_area = block.inner(area);

        // NOTE: We only reliably know the size of the area for the editor once we arrive at this point.
        // Calling the resize_width will cause the visual blocks to be populated in the state.
        // If width or height is not changed between frames, the resize_width is a noop.
        state.resize_viewport(inner_area.as_size());

        state.update_layout();

        let mut lines = state.virtual_document.meta().to_vec();
        lines.extend(state.virtual_document.lines().to_vec());

        let visible_lines = lines
            .iter()
            .skip(state.viewport().top() as usize)
            .take(state.viewport().bottom() as usize)
            // Cheaper to clone the subset of the lines
            .cloned()
            .map(|visual_line| visual_line.into())
            .collect::<Vec<Line>>();

        let rendered_lines_count = state.virtual_document.lines().len();
        let meta_lines_count = state.virtual_document.meta().len();

        Paragraph::new(visible_lines).block(block).render(area, buf);

        if !state.content.is_empty() {
            CursorWidget::default()
                .with_offset(Offset {
                    x: inner_area.x as i32,
                    y: inner_area.y as i32 + meta_lines_count as i32,
                })
                .render(state.viewport().area(), buf, &mut state.cursor);
        }

        if !area.is_empty() && lines.len() as u16 > inner_area.bottom() {
            let mut scroll_state =
                ScrollbarState::new(rendered_lines_count).position(state.cursor.virtual_row());

            Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                area,
                buf,
                &mut scroll_state,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::note_editor::state::EditMode;

    use super::*;
    use indoc::indoc;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

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
            let mut state = NoteEditorState::new(text, "Test", Path::new("test.md"));
            terminal
                .draw(|frame| {
                    NoteEditor::default().render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            assert_snapshot!(terminal.backend());
        });
    }

    #[test]
    fn test_rendered_editor_states() {
        type TestCase = (&'static str, Box<dyn Fn(Rect) -> NoteEditorState<'static>>);

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

        let tests: Vec<TestCase> = vec![
            (
                "empty_default_state",
                Box::new(|_| NoteEditorState::default()),
            ),
            (
                "read_mode_with_content",
                Box::new(|_| NoteEditorState::new(content, "Test", Path::new("test.md"))),
            ),
            (
                "edit_mode_with_content",
                Box::new(|_| {
                    let mut state = NoteEditorState::new(content, "Test", Path::new("test.md"));
                    state.set_view(View::Edit(EditMode::Source));
                    state
                }),
            ),
            (
                "edit_mode_with_content_and_simple_change",
                Box::new(|area| {
                    let mut state = NoteEditorState::new(content, "Test", Path::new("test.md"));
                    state.resize_viewport(area.as_size());
                    state.set_view(View::Edit(EditMode::Source));
                    state.insert_char('#');
                    state.exit_insert();
                    state.set_view(View::Read);
                    state
                }),
            ),
            (
                "edit_mode_with_arbitrary_cursor_move",
                Box::new(|area| {
                    let mut state = NoteEditorState::new(content, "Test", Path::new("test.md"));
                    state.resize_viewport(area.as_size());
                    state.set_view(View::Edit(EditMode::Source));
                    state.cursor_right(7);
                    state.insert_char(' ');
                    state.insert_char('B');
                    state.insert_char('a');
                    state.insert_char('s');
                    state.insert_char('a');
                    state.insert_char('l');
                    state.insert_char('t');
                    state.exit_insert();
                    state.set_view(View::Read);
                    state
                }),
            ),
            (
                "edit_mode_with_content_with_complete_word_input_change",
                Box::new(|area| {
                    let mut state = NoteEditorState::new(content, "Test", Path::new("test.md"));
                    state.resize_viewport(area.as_size());
                    state.cursor_down(1);
                    state.set_view(View::Edit(EditMode::Source));
                    state.insert_char('\n');
                    state.insert_char('B');
                    state.insert_char('a');
                    state.insert_char('s');
                    state.insert_char('a');
                    state.insert_char('l');
                    state.insert_char('t');
                    state.insert_char('\n');
                    state.insert_char('\n');
                    state.exit_insert();
                    state.set_view(View::Read);
                    state
                }),
            ),
        ];

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();

        tests.into_iter().for_each(|(name, state_fn)| {
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    let mut state = state_fn(frame.area());
                    NoteEditor::default().render(frame.area(), frame.buffer_mut(), &mut state)
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
            let mut state = NoteEditorState::new(content, name, Path::new("test.md"));
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    NoteEditor::default().render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }
}
