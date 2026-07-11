use std::{marker::PhantomData, ops::Range};

use ratatui::{
    buffer::Buffer,
    layout::{Offset, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{
        Block, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};
use unicode_width::UnicodeWidthChar;

use crate::note_editor::{
    cursor::CursorWidget,
    state::{NoteEditorState, SelectionMode, View},
    viewport::Viewport,
    virtual_document::VirtualLine,
};

const SELECTION_STYLE: Style = Style::new().reversed();
const YANK_FLASH_STYLE: Style = Style::new().bg(Color::LightCyan);

fn render_highlight(
    buf: &mut Buffer,
    inner_area: Rect,
    viewport: &Viewport,
    lines: &[VirtualLine],
    meta_len: usize,
    range: &Range<usize>,
    style: Style,
) {
    let viewport_top = viewport.top() as usize;
    let horizontal_scroll = viewport.left();

    let paint = |buf: &mut Buffer, col: u16, y: u16, width: u16| {
        if col >= horizontal_scroll {
            let cell = Rect::new(inner_area.x + col - horizontal_scroll, y, width.max(1), 1)
                .intersection(inner_area);
            buf.set_style(cell, style);
        }
    };

    lines
        .iter()
        .enumerate()
        .skip(viewport_top)
        .take(inner_area.height as usize)
        .filter(|(idx, _)| *idx >= meta_len)
        .for_each(|(idx, line)| {
            let y = inner_area.y + (idx - viewport_top) as u16;
            let spans = line.virtual_spans();
            let mut col = 0u16;

            for (i, span) in spans.iter().enumerate() {
                match span.source_range() {
                    Some(source_range) => {
                        span.char_indices().fold(col, |col, (byte_idx, ch)| {
                            let width = ch.width().unwrap_or(0) as u16;
                            if range.contains(&(source_range.start + byte_idx)) {
                                paint(buf, col, y, width);
                            }
                            col + width
                        });
                    }
                    // A synthetic span (rendered list marker, prefix, quote glyph)
                    // stands in for source it does not carry. Highlight it when the
                    // content it precedes is selected, so a selected line's marker
                    // is not left blank.
                    None => {
                        let precedes_selection = spans[i + 1..]
                            .iter()
                            .find_map(|span| span.source_range())
                            .is_some_and(|next| range.contains(&next.start));
                        if precedes_selection {
                            paint(buf, col, y, span.width() as u16);
                        }
                    }
                }
                col += span.width() as u16;
            }
        });
}

#[derive(Default)]
pub struct NoteEditor<'a>(pub PhantomData<&'a ()>);

/// The text area inside the editor's border and padding. Must match the block
/// built in [`NoteEditor::render`] so image overlays line up with the text.
pub fn inner_area(area: Rect) -> Rect {
    Block::bordered()
        .padding(Padding::horizontal(1))
        .inner(area)
}

impl<'a> StatefulWidget for NoteEditor<'a> {
    type State = NoteEditorState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (mode_label, mode_color) = match state.view {
            View::Edit(..) if state.vim_mode() && state.insert_mode() => ("INSERT", Color::Green),
            View::Edit(..) if state.vim_mode() && state.is_selecting() => {
                match state.selection().map(|selection| selection.mode) {
                    Some(SelectionMode::Line) => ("V-LINE", Color::Magenta),
                    _ => ("VISUAL", Color::Magenta),
                }
            }
            View::Edit(..) if state.vim_mode() => ("NORMAL", Color::Yellow),
            View::Edit(..) => ("EDIT", Color::Green),
            View::Read => ("READ", Color::Red),
        };

        let block = Block::bordered()
            .border_type(if state.active() {
                state.symbols.border_active.into()
            } else {
                state.symbols.border_inactive.into()
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
                    format!(" {mode_label}").fg(mode_color).bold().italic(),
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

        let meta_lines_count = state.virtual_document.meta().len();

        Paragraph::new(visible_lines)
            .scroll((0, state.viewport().left()))
            .block(block)
            .render(area, buf);

        if let Some(range) = state.selection_range() {
            render_highlight(
                buf,
                inner_area,
                state.viewport(),
                &lines,
                meta_lines_count,
                &range,
                SELECTION_STYLE,
            );
        }

        if let Some(range) = state.yank_flash_range() {
            render_highlight(
                buf,
                inner_area,
                state.viewport(),
                &lines,
                meta_lines_count,
                &range,
                YANK_FLASH_STYLE,
            );
        }

        if !state.content.is_empty() || state.is_editing() {
            CursorWidget::default()
                .with_offset(Offset {
                    x: inner_area.x as i32,
                    y: inner_area.y as i32,
                })
                .with_meta_len(meta_lines_count as u16)
                .render(state.viewport().area(), buf, &mut state.cursor);
        }

        if !area.is_empty() && lines.len() as u16 > inner_area.bottom() {
            // Track the scroll offset, not the cursor: the cursor pins to a
            // trailing image's top row while the viewport scrolls on through it.
            // Length is the number of scroll positions so the thumb reaches the
            // bottom exactly when the last row is on screen.
            let max_top = lines.len().saturating_sub(inner_area.height as usize);
            let mut scroll_state =
                ScrollbarState::new(max_top + 1).position(state.viewport().top() as usize);

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
    use std::collections::{HashMap, HashSet};
    use std::path::{Path, PathBuf};

    use crate::{
        config::Symbols,
        image::ImageKey,
        note_editor::{ast::ImageSource, state::EditMode},
    };

    use super::*;
    use indoc::indoc;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, layout::Size, Terminal};

    #[test]
    fn test_image_reserves_space() {
        let content = indoc! { r#"## Diagram

            ![[diagram.png]]

            Text after the image.
            "#};

        let mut state =
            NoteEditorState::new(content, "Test", Path::new("test.md"), &Symbols::unicode());

        // Simulate the app having resolved and decoded the embed: a 200x100px
        // image at a 10x20 font cell scaled to fill the content width, preserving
        // its 2:1 aspect ratio and bounded by the image_max_height ratio.
        let source = ImageSource::Embed("diagram.png".to_string());
        let key = ImageKey::Path(PathBuf::from("/vault/diagram.png"));
        state.set_image_meta((10, 20), Some(0.8));
        state.set_image_state(
            HashMap::from([(source, key.clone())]),
            HashMap::from([(key, (200, 100))]),
            HashSet::new(),
        );

        let mut terminal = Terminal::new(TestBackend::new(40, 14)).unwrap();
        terminal
            .draw(|frame| {
                NoteEditor::default().render(frame.area(), frame.buffer_mut(), &mut state)
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn scrolls_to_reveal_a_trailing_image() {
        // A tall image as the last block reserves more rows than the cursor can
        // reach; scrolling to the end must reveal it down to the document's edge.
        let content = indoc! { r#"# Title

            Some text.

            ![[tall.png]]
            "#};

        let mut state =
            NoteEditorState::new(content, "Test", Path::new("test.md"), &Symbols::unicode());

        let source = ImageSource::Embed("tall.png".to_string());
        let key = ImageKey::Path(PathBuf::from("/vault/tall.png"));
        state.set_image_meta((10, 20), Some(3.0));
        state.set_image_state(
            HashMap::from([(source, key.clone())]),
            HashMap::from([(key, (100, 400))]),
            HashSet::new(),
        );
        state.resize_viewport(Size::new(40, 10));

        let doc_height = state.virtual_document.meta().len() + state.virtual_document.lines().len();
        let max_top = doc_height.saturating_sub(10);
        assert!(max_top > 0, "the image should exceed the viewport height");

        assert_eq!(state.viewport().y, 0);
        state.cursor_to_end();
        assert_eq!(state.viewport().y as usize, max_top);
    }

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
            let mut state =
                NoteEditorState::new(text, "Test", Path::new("test.md"), &Symbols::unicode());
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
                Box::new(|_| {
                    NoteEditorState::new(content, "Test", Path::new("test.md"), &Symbols::unicode())
                }),
            ),
            (
                "edit_mode_with_content",
                Box::new(|_| {
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.set_view(View::Edit(EditMode::Source));
                    state
                }),
            ),
            (
                "edit_mode_with_content_and_simple_change",
                Box::new(|area| {
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
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
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
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
                "edit_mode_task_list_then_multiple_empty_lines",
                Box::new(|_| {
                    let content = indoc! { r#"## Tasks
                    - [ ] one
                    - [ ] two



                    next paragraph
                    "#};
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.set_view(View::Edit(EditMode::Source));
                    state
                }),
            ),
            (
                "edit_mode_typing_newline_in_active_block_stable_trailing",
                Box::new(|area| {
                    let content = "para1\n\npara2\n";
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.resize_viewport(area.as_size());
                    state.set_view(View::Edit(EditMode::Source));
                    state.cursor_right(5);
                    state.insert_char('\n');
                    state.insert_char('\n');
                    state
                }),
            ),
            (
                "edit_mode_no_empty_line_between_adjacent_blocks",
                Box::new(|_| {
                    let content = indoc! { r#"## Heading
                    Paragraph immediately under heading.
                    - first item
                    - second item
                    "#};
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.set_view(View::Edit(EditMode::Source));
                    state
                }),
            ),
            (
                "edit_mode_preserves_loose_list_empty_lines",
                Box::new(|_| {
                    let content = indoc! { r#"## Lists with line breaks

                    1. First list item

                    2. Second list item
                    3. Third list item

                    4. Fourth list item
                    "#};
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.set_view(View::Edit(EditMode::Source));
                    state
                }),
            ),
            (
                "edit_mode_with_content_with_complete_word_input_change",
                Box::new(|area| {
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
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
            (
                // A table edits as a box; only the cursor's row reveals raw. Here
                // the cursor is on the header row.
                "edit_mode_table_cursor_header",
                Box::new(|area| {
                    let content = indoc! { r#"## Tables

                    | Name  | Role       |
                    | :---- | :--------: |
                    | Alice | Maintainer |
                    | Bob   | Reviewer   |
                    "#};
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.resize_viewport(area.as_size());
                    state.set_view(View::Edit(EditMode::Source));
                    state.cursor_down(1);
                    state
                }),
            ),
            (
                // A broken table (invalid delimiter row) is edited fully raw so the
                // broken markdown stays visible and fixable — no box hides it, even
                // with the cursor away from the broken line.
                "edit_mode_table_broken_is_raw",
                Box::new(|area| {
                    let content = indoc! { r#"## Tables

                    | Name  | Role       |
                    | :xx-- | :--------: |
                    | Alice | Maintainer |
                    | Bob   | Reviewer   |
                    "#};
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.resize_viewport(area.as_size());
                    state.set_view(View::Edit(EditMode::Source));
                    state.cursor_down(1);
                    state
                }),
            ),
            (
                // Breaking a live table by deleting a delimiter column (so the
                // delimiter no longer matches the header) drops it out of table
                // syntax. Even though the cursor moves away from the broken line, the
                // whole block falls back to raw so it stays visible and fixable.
                "edit_mode_table_break_column_count",
                Box::new(|area| {
                    let content = indoc! { r#"## Tables

                    | Name | Role |
                    | ---- | ---- |
                    | A    | B    |
                    "#};
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.resize_viewport(area.as_size());
                    state.set_view(View::Edit(EditMode::Source));
                    // Onto the delimiter row, then delete its second column.
                    state.cursor_down(1);
                    state.cursor_down(1);
                    state.cursor_right(40);
                    for _ in 0..7 {
                        state.delete_char();
                    }
                    // Move the cursor back up to the header, away from the break.
                    state.cursor_up(1);
                    state
                }),
            ),
            (
                // The delimiter row is reachable too: landing on it reveals it raw
                // so its alignment markers can be edited.
                "edit_mode_table_cursor_delimiter",
                Box::new(|area| {
                    let content = indoc! { r#"## Tables

                    | Name  | Role       |
                    | :---- | :--------: |
                    | Alice | Maintainer |
                    | Bob   | Reviewer   |
                    "#};
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.resize_viewport(area.as_size());
                    state.set_view(View::Edit(EditMode::Source));
                    state.cursor_down(1);
                    state.cursor_down(1);
                    state
                }),
            ),
            (
                // Stepping down reveals a body row raw while the rest stays boxed.
                "edit_mode_table_cursor_body",
                Box::new(|area| {
                    let content = indoc! { r#"## Tables

                    | Name  | Role       |
                    | :---- | :--------: |
                    | Alice | Maintainer |
                    | Bob   | Reviewer   |
                    "#};
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.resize_viewport(area.as_size());
                    state.set_view(View::Edit(EditMode::Source));
                    state.cursor_down(1);
                    state.cursor_down(1);
                    state.cursor_down(1);
                    state
                }),
            ),
            (
                // Only the list item under the cursor should render raw; the
                // surrounding items stay rendered. Ref: issue #486.
                "edit_mode_list_line_by_line_raw",
                Box::new(|area| {
                    let content = indoc! { r#"## Shopping

                    - apples
                    - bananas
                    - cherries
                    "#};
                    let mut state = NoteEditorState::new(
                        content,
                        "Test",
                        Path::new("test.md"),
                        &Symbols::unicode(),
                    );
                    state.resize_viewport(area.as_size());
                    state.set_view(View::Edit(EditMode::Source));
                    // Enter the list (lands on the first item), then step down to
                    // the "bananas" item.
                    state.cursor_down(1);
                    state.cursor_down(1);
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
                "tables",
                indoc! { r#"## Tables
                Columns size to their content and wrap long text so the table fits.

                | Name  | Role       | Notes                                                            |
                | :---- | :--------: | ---------------------------------------------------------------: |
                | Alice | Maintainer | Writes most of the core code and reviews incoming pull requests. |
                | Bob   | Reviewer   | Short note.                                                      |
                "#},
            ),
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
            let mut state =
                NoteEditorState::new(content, name, Path::new("test.md"), &Symbols::unicode());
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    NoteEditor::default().render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }

    #[test]
    fn test_selected_list_marker_is_highlighted() {
        use ratatui::{layout::Size, style::Modifier};

        let mut state = NoteEditorState::new(
            "- alpha\n- beta\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.set_vim_mode(true);
        state.set_editor_enabled(true);
        state.resize_viewport(Size::new(40, 10));
        state.set_view(View::Edit(EditMode::Source));

        // Linewise-select from the first item down onto the second. The cursor
        // lands on line two (rendered raw), so line one keeps its prettified
        // "●" marker while sitting inside the selection.
        state.toggle_selection(SelectionMode::Line);
        state.cursor_down(1);

        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        terminal
            .draw(|frame| {
                NoteEditor::default().render(frame.area(), frame.buffer_mut(), &mut state)
            })
            .unwrap();

        let highlighted = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.symbol() == "●" && cell.modifier.contains(Modifier::REVERSED));

        assert!(
            highlighted,
            "the prettified list marker on a selected line should be highlighted"
        );
    }
}
