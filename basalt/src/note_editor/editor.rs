use std::{fmt, marker::PhantomData, ops::Deref};

use ratatui::{
    buffer::Buffer,
    layout::{Offset, Rect, Size},
    style::{Color, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget,
    },
};

use crate::note_editor::{
    ast,
    cursor::{Cursor, CursorWidget},
    parser,
    virtual_document::VirtualDocument,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum EditMode {
    #[default]
    /// Shows the markdown exactly as written
    Source,
    // TODO:
    // /// Hides most of the markdown syntax
    // LivePreview
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum View {
    #[default]
    Read,
    Edit(EditMode),
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            View::Read => write!(f, "READ"),
            View::Edit(..) => write!(f, "EDIT"),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Viewport {
    area: Rect,
}

impl Deref for Viewport {
    type Target = Rect;

    fn deref(&self) -> &Self::Target {
        &self.area
    }
}

impl Viewport {
    pub fn resize(&mut self, size: Size) {
        self.area.width = size.width;
        self.area.height = size.height;
    }

    pub fn scroll_by(&mut self, offset: (i32, i32)) {
        self.area = self.offset(Offset {
            y: offset.0,
            x: offset.1,
        })
    }
}

// TODO: Some Editable block when user hits insert?
#[derive(Clone, Debug, Default)]
pub struct State<'a> {
    // TODO: Use Rope instead of String for O(log n) instead of O(n).
    content: String,
    pub view: View,
    pub cursor: Cursor,
    filename: String,
    pub ast_nodes: Vec<ast::Node>,
    virtual_document: VirtualDocument<'a>,
    active: bool,
    modified: bool,
    viewport: Viewport,
}

impl<'a> State<'a> {
    pub fn new(content: &str, filename: &str) -> Self {
        let ast_nodes = parser::from_str(content);
        let content = content.to_string();
        Self {
            content: content.clone(),
            view: View::Read,
            cursor: Cursor::default(),
            viewport: Viewport::default(),
            virtual_document: VirtualDocument::default(),
            filename: filename.to_string(),
            ast_nodes,
            active: false,
            modified: false,
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn current_row(&self) -> usize {
        *self
            .virtual_document
            .line_to_block()
            .get(self.cursor.virtual_line())
            .unwrap_or(&0)
    }

    pub fn scroll_by(&mut self, offset: (i32, i32)) {
        self.viewport.scroll_by(offset);
    }

    pub fn set_view(&mut self, view: View) {
        self.view = view;

        self.virtual_document.layout(
            &self.filename,
            &self.content,
            &self.view,
            self.cursor.virtual_line(),
            &self.ast_nodes,
            self.viewport.width.into(),
        );

        match view {
            View::Read => self.cursor.enter_read_mode(&self.virtual_document),
            View::Edit(..) => self.cursor.enter_edit_mode(&self.virtual_document),
        }
    }

    pub fn resize_viewport(&mut self, size: Size) {
        // If width has changed we need to recalculate the wrapped lines.
        // Height change doesn't matter as it only affects what is visible.
        if self.viewport.width != size.width {
            self.virtual_document.layout(
                &self.filename,
                &self.content,
                &self.view,
                self.cursor.virtual_line(),
                &self.ast_nodes,
                size.width.into(),
            );
        }

        // TODO: if height has changed we need to move cursor if it goes out of bounds This means
        // we need to move it to last visible spot. We only need to care about the bottom()
        // boundary.
        self.viewport.resize(size);
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn cursor_left(&mut self) {}

    pub fn cursor_right(&mut self) {}

    // TODO: Applies to both cursor_up and cursor_down
    // The cursor should always be fixed to the viewport. This would enable easier implementation
    // for e.g. search feature when navigating between matches
    pub fn cursor_up(&mut self, amount: usize) {
        let prevline = self.cursor.virtual_line();
        self.cursor.cursor_up(amount, self.virtual_document.lines());

        let diff = self.cursor.virtual_line() as i32 - prevline as i32;

        if self.cursor.virtual_line() < self.viewport.top() as usize {
            self.viewport.scroll_by((diff, 0));
        }
    }

    pub fn cursor_down(&mut self, amount: usize) {
        // TODO: Implement scroll off so that the note scroll offset can be changed by moving
        // cursor downwards when we are at the bottom.
        let prevline = self.cursor.virtual_line();
        self.cursor
            .cursor_down(amount, self.virtual_document.lines());

        let diff = self.cursor.virtual_line() as i32 - prevline as i32;

        if self.cursor.virtual_line()
            >= self
                .viewport
                .bottom()
                .saturating_sub(self.virtual_document.meta().len() as u16) as usize
        {
            self.viewport.scroll_by((diff, 0));
        }
    }
}

#[derive(Default)]
pub struct NoteEditor<'a>(pub PhantomData<&'a ()>);

impl<'a> StatefulWidget for NoteEditor<'a> {
    type State = State<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mode_color = match state.view {
            View::Edit(..) => Color::Green,
            View::Read => Color::Red,
        };

        let block = Block::bordered()
            .border_type(if state.active {
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

        // We only reliable know the size of the area for the editor once we arrive at this point.
        // Calling the resize_width will cause the visual_blocks to be populated in the state.
        // If width is not changed between frames the resize_width is a noop.
        state.resize_viewport(inner_area.as_size());

        let mut lines = state.virtual_document.meta().to_vec();
        lines.extend(state.virtual_document.lines().to_vec());

        let visible_lines = lines
            .iter()
            .skip(state.viewport.top() as usize)
            .take(state.viewport.bottom() as usize)
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
                .render(state.viewport.area, buf, &mut state.cursor);
        }

        if !area.is_empty() && lines.len() as u16 > inner_area.bottom() {
            let mut scroll_state =
                ScrollbarState::new(rendered_lines_count).position(state.cursor.virtual_line());

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
    use crate::note_editor::editor::EditMode;

    use super::*;
    use indoc::indoc;
    use insta::assert_snapshot;
    use ratatui::{
        backend::TestBackend,
        // crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
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
            let mut state = State::new(text, "Test");
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
            ("empty_default_state", State::default()),
            ("with_content", State::new(content, "Test")),
            ("read_mode_with_content", {
                let mut state = State::new(content, "Test");
                state.set_view(View::Read);
                state
            }),
            ("edit_mode_with_content", {
                let mut state = State::new(content, "Test");
                state.set_view(View::Edit(EditMode::Source));
                state
            }),
            // ("edit_mode_with_content_and_simple_change", {
            //     let mut state = State::new(content, "Test");
            //     state.set_view(View::Edit(EditMode::Source));
            //     state.edit(KeyEvent::new(KeyCode::Char('#'), KeyModifiers::empty()).into());
            //     state.exit_insert();
            //     state.set_view(View::Read);
            //     state
            // }),
            // ("edit_mode_with_arbitrary_cursor_move", {
            //     let mut state = State::new(content, "Test");
            //     state.set_content(content);
            //     state.cursor_move_col(7);
            //     state.set_view(View::Edit(EditMode::Source));
            //     state.edit(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('B'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()).into());
            //     state.exit_insert();
            //     state.set_view(View::Read);
            //     state
            // }),
            // ("edit_mode_with_content_with_complete_word_input_change", {
            //     let mut state = State::new(content, "Test");
            //     state.set_content(content);
            //     state.cursor_down();
            //     state.set_view(View::Edit(EditMode::Source));
            //     state.edit(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('B'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()).into());
            //     state.edit(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()).into());
            //     state.exit_insert();
            //     state.set_view(View::Read);
            //     state
            // }),
        ];

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();

        tests.into_iter().for_each(|(name, mut state)| {
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
            let mut state = State::new(content, name);
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
