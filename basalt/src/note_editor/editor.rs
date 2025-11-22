use std::{
    fmt,
    marker::PhantomData,
    ops::Deref,
    path::{Path, PathBuf},
};

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
    ast::{self, SourceRange},
    cursor::{self, Cursor, CursorWidget},
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
    pub fn size_changed(&self, size: Size) -> bool {
        (self.height, self.width) != (size.height, size.width)
    }

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

#[derive(Clone, Debug)]
pub struct TextBuffer {
    // TODO: Change to Rope
    pub content: String,
    pub source_range: SourceRange<usize>,
    origina_source_range: SourceRange<usize>,
    modified: bool,
}

impl TextBuffer {
    pub fn new(content: &str, source_range: SourceRange<usize>) -> Self {
        Self {
            content: content.to_string(),
            origina_source_range: source_range.clone(),
            source_range,
            // TODO: Implement history to get accurate modified bool
            modified: false,
        }
    }

    pub fn insert_char(&mut self, c: char, idx: usize) {
        let char_idx = idx.saturating_sub(self.source_range.start);

        let byte_idx = self
            .content
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.content.len());

        self.content.insert(byte_idx, c);
        self.source_range.end += 1;
        self.modified = true;
    }

    pub fn delete_char(&mut self, idx: usize) {
        let char_idx = idx.saturating_sub(self.source_range.start);
        if let Some((byte_idx, _)) = self.content.char_indices().nth(char_idx) {
            self.content.remove(byte_idx);
            self.source_range.end = self.source_range.end.saturating_sub(1);
            self.modified = true;
        }
    }

    pub fn write(&self, original_content: &str) -> String {
        if self.modified {
            let str_start = &original_content[..self.origina_source_range.start];
            let str_end = &original_content[self.origina_source_range.end..];
            format!("{}{}{}", str_start, self.content, str_end)
        } else {
            original_content.to_owned()
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct State<'a> {
    // TODO: Use Rope instead of String for O(log n) instead of O(n).
    pub content: String,
    pub view: View,
    pub cursor: Cursor,
    _filepath: PathBuf,
    filename: String,
    pub ast_nodes: Vec<ast::Node>,
    virtual_document: VirtualDocument<'a>,
    active: bool,
    _modified: bool,
    viewport: Viewport,
    edit_buffer: Option<TextBuffer>,
}

impl<'a> State<'a> {
    pub fn new(content: &str, filename: &str, filepath: &Path) -> Self {
        let ast_nodes = parser::from_str(content);
        let content = content.to_string();
        Self {
            edit_buffer: None,
            content: content.clone(),
            view: View::Read,
            cursor: Cursor::default(),
            viewport: Viewport::default(),
            virtual_document: VirtualDocument::default(),
            filename: filename.to_string(),
            _filepath: filepath.to_path_buf(),
            ast_nodes,
            active: false,
            _modified: false,
        }
    }

    pub fn is_editing(&self) -> bool {
        matches!(self.view, View::Edit(..))
    }

    pub fn edit_buffer(&self) -> Option<&TextBuffer> {
        self.edit_buffer.as_ref()
    }

    // FIXME: if document is empty cannot write as there is no markdown block to write on.
    pub fn enter_insert(&mut self, block_idx: usize) {
        if let Some((_, block)) = self.virtual_document.get_block(block_idx) {
            let source_range = block.source_range();
            if let Some(content) = self.content.get(source_range.clone()) {
                self.edit_buffer = Some(TextBuffer::new(content, source_range.clone()));
            }
        }
    }

    pub fn exit_insert(&mut self) {
        if matches!(self.view, View::Read) {
            return;
        }

        if let Some(buffer) = self.edit_buffer() {
            let new_content = buffer.write(&self.content);
            if self.content != new_content {
                self.content = new_content;
                self.ast_nodes = parser::from_str(&self.content);
                self.update_layout();
            }
        }

        self.edit_buffer = None;
    }

    pub fn insert_char(&mut self, c: char) {
        if let Some(buffer) = &mut self.edit_buffer {
            buffer.insert_char(c, self.cursor.source_offset());
            self.update_layout();
            self.cursor_right(1);
        }
    }

    pub fn delete_char(&mut self) {
        if let Some(buffer) = &mut self.edit_buffer {
            buffer.delete_char(self.cursor.source_offset().saturating_sub(1));
            self.update_layout();
            self.cursor_left(1);
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn current_block(&self) -> usize {
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
        let block_idx = self.current_block();

        self.view = view;

        if matches!(self.view, View::Edit(..)) {
            self.enter_insert(block_idx);
        }
        //
        // self.virtual_document.layout(
        //     &self.filename,
        //     &self.content,
        //     &self.view,
        //     Some(block_idx),
        //     &self.ast_nodes,
        //     self.viewport.width.into(),
        //     self.edit_buffer.clone(),
        // );

        match self.view {
            View::Read => {
                self.exit_insert();
                cursor::update(
                    &cursor::Message::SwitchMode(cursor::CursorMode::Read),
                    &mut self.cursor,
                    self.virtual_document.lines(),
                    &None,
                );
            }
            View::Edit(..) => {
                cursor::update(
                    &cursor::Message::SwitchMode(cursor::CursorMode::Edit),
                    &mut self.cursor,
                    self.virtual_document.lines(),
                    &self.edit_buffer,
                );
            }
        }
    }

    pub fn resize_viewport(&mut self, size: Size) {
        // TODO: if height has changed we need to move cursor if it goes out of bounds This means
        // we need to move it to last visible spot. We only need to care about the bottom()
        // boundary.
        if self.viewport.size_changed(size) {
            let current_block_idx =
                matches!(self.view, View::Edit(..)).then_some(self.current_block());

            self.virtual_document.layout(
                &self.filename,
                &self.content,
                &self.view,
                current_block_idx,
                &self.ast_nodes,
                size.width.into(),
                self.edit_buffer.clone(),
            );

            self.viewport.resize(size);
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn modified(&self) -> bool {
        self.edit_buffer()
            .map(|buffer| buffer.modified)
            .unwrap_or(false)
    }

    pub fn cursor_left(&mut self, amount: usize) {
        cursor::update(
            &cursor::Message::MoveLeft(amount),
            &mut self.cursor,
            self.virtual_document.lines(),
            &self.edit_buffer,
        );
    }

    pub fn cursor_right(&mut self, amount: usize) {
        cursor::update(
            &cursor::Message::MoveRight(amount),
            &mut self.cursor,
            self.virtual_document.lines(),
            &self.edit_buffer,
        );
    }

    // TODO: Implement cursor jumping
    // pub fn cursor_jump(&mut self, jump_to_row: u16) {
    //     if matches!(self.cursor.mode(), CursorMode::Read) {
    //         self.cursor.move_action(
    //             CursorMove::Jump(jump_to_row, 0),
    //             self.virtual_document.lines(),
    //             &self.edit_buffer,
    //         );
    //     }
    // }

    pub fn update_layout(&mut self) {
        let current_block_idx = if matches!(self.view, View::Edit(..)) {
            Some(self.current_block())
        } else {
            None
        };

        self.virtual_document.layout(
            &self.filename,
            &self.content,
            &self.view,
            current_block_idx,
            &self.ast_nodes,
            self.viewport.area.width.into(),
            self.edit_buffer.clone(),
        );
    }

    // TODO: Applies to both cursor_up and cursor_down
    // The cursor should always be fixed to the viewport. This would enable easier implementation
    // for e.g. search feature when navigating between matches
    pub fn cursor_up(&mut self, amount: usize) {
        let prev_line = self.cursor.virtual_line();

        // If in edit mode, temporarily switch to all-visual layout before moving
        // so that line_to_block mapping is accurate
        if matches!(self.view, View::Edit(..)) {
            self.exit_insert();
            self.virtual_document.layout(
                &self.filename,
                &self.content,
                &self.view,
                None, // All blocks in visual mode
                &self.ast_nodes,
                self.viewport.area.width.into(),
                None,
            );
        }

        cursor::update(
            &cursor::Message::MoveUp(amount),
            &mut self.cursor,
            self.virtual_document.lines(),
            &self.edit_buffer,
        );

        if matches!(self.view, View::Edit(..)) {
            let current_block_idx = self.current_block();

            self.enter_insert(current_block_idx);

            self.virtual_document.layout(
                &self.filename,
                &self.content,
                &self.view,
                Some(current_block_idx),
                &self.ast_nodes,
                self.viewport.area.width.into(),
                self.edit_buffer.clone(),
            );
        }

        let diff = self.cursor.virtual_line() as i32 - prev_line as i32;

        if self.cursor.virtual_line() < self.viewport.top() as usize {
            self.viewport.scroll_by((diff, 0));
        }
    }

    pub fn cursor_down(&mut self, amount: usize) {
        // TODO: Implement scroll off so that the note scroll offset can be changed by moving
        // cursor downwards when we are at the bottom.
        let prev_line = self.cursor.virtual_line();

        // If in edit mode, temporarily switch to all-visual layout before moving
        // so that line_to_block mapping is accurate
        if matches!(self.view, View::Edit(..)) {
            self.exit_insert();
            self.virtual_document.layout(
                &self.filename,
                &self.content,
                &self.view,
                None, // All blocks in visual mode
                &self.ast_nodes,
                self.viewport.area.width.into(),
                None,
            );
        }

        cursor::update(
            &cursor::Message::MoveDown(amount),
            &mut self.cursor,
            self.virtual_document.lines(),
            &self.edit_buffer,
        );

        // Re-layout if we're in edit mode
        if matches!(self.view, View::Edit(..)) {
            let current_block_idx = self.current_block();

            // Enter insert mode for the current block
            self.enter_insert(current_block_idx);

            self.virtual_document.layout(
                &self.filename,
                &self.content,
                &self.view,
                Some(current_block_idx),
                &self.ast_nodes,
                self.viewport.area.width.into(),
                self.edit_buffer.clone(),
            );
        }

        let diff = self.cursor.virtual_line() as i32 - prev_line as i32;

        if self.cursor.virtual_line()
            >= self
                .viewport
                .bottom()
                .saturating_sub(self.virtual_document.meta().len() as u16) as usize
        {
            self.viewport.scroll_by((diff, 0));
        }

        // if self.cursor.virtual_column() >= self.viewport.width.into() {
        //     self.viewport.scroll_by((
        //         0,
        //         self.viewport
        //             .width
        //             .saturating_sub(self.cursor.virtual_column() as u16) as i32,
        //     ));
        // }
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
            let mut state = State::new(text, "Test", Path::new("test.md"));
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
        type TestCase = (&'static str, Box<dyn Fn(Rect) -> State<'static>>);

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
            ("empty_default_state", Box::new(|_| State::default())),
            (
                "read_mode_with_content",
                Box::new(|_| State::new(content, "Test", Path::new("test.md"))),
            ),
            (
                "edit_mode_with_content",
                Box::new(|_| {
                    let mut state = State::new(content, "Test", Path::new("test.md"));
                    state.set_view(View::Edit(EditMode::Source));
                    state
                }),
            ),
            (
                "edit_mode_with_content_and_simple_change",
                Box::new(|area| {
                    let mut state = State::new(content, "Test", Path::new("test.md"));
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
                    let mut state = State::new(content, "Test", Path::new("test.md"));
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
                    let mut state = State::new(content, "Test", Path::new("test.md"));
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
            let mut state = State::new(content, name, Path::new("test.md"));
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    NoteEditor::default().render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }

    // TODO: Add snapshots tests for raw rendering (edit buffer)
    #[test]
    fn test_raw_render() {
        let tests = [
            // (
            //     "paragraphs",
            //     indoc! { r#"## Paragraphs
            //     To create paragraphs in Markdown, use a **blank line** to separate blocks of text. Each block of text separated by a blank line is treated as a distinct paragraph.
            //
            //     This is a paragraph.
            //
            //     This is another paragraph.
            //
            //     A blank line between lines of text creates separate paragraphs. This is the default behavior in Markdown.
            //     "#},
            // ),
            // (
            //     "headings",
            //     indoc! { r#"## Headings
            //     To create a heading, add up to six `#` symbols before your heading text. The number of `#` symbols determines the size of the heading.
            //
            //     # This is a heading 1
            //     ## This is a heading 2
            //     ### This is a heading 3
            //     #### This is a heading 4
            //     ##### This is a heading 5
            //     ###### This is a heading 6
            //     "#},
            // ),
            // (
            //     "lists",
            //     indoc! { r#"## Lists
            //     You can create an unordered list by adding a `-`, `*`, or `+` before the text.
            //
            //     - First list item
            //     - Second list item
            //     - Third list item
            //
            //     To create an ordered list, start each line with a number followed by a `.` or `)` symbol.
            //
            //     1. First list item
            //     2. Second list item
            //     3. Third list item
            //
            //     1) First list item
            //     2) Second list item
            //     3) Third list item
            //     "#},
            // ),
            // (
            //     "lists_raw_line_breaks",
            //     indoc! { r#"## Lists with line breaks
            //     You can use line breaks within an ordered list without altering the numbering.
            //
            //     1. First list item
            //
            //
            //       2. Second list item
            //        3. Third list item
            //
            //       4. Fourth list item
            //
            //     5. Fifth list item
            //       6. Sixth list item
            //     "#},
            // ),
            // (
            //     "task_lists",
            //     indoc! { r#"## Task lists
            //     To create a task list, start each list item with a hyphen and space followed by `[ ]`.
            //
            //     - [x] This is a completed task.
            //     - [ ] This is an incomplete task.
            //
            //     You can toggle a task in Reading view by selecting the checkbox.
            //
            //     > [!tip]
            //     > You can use any character inside the brackets to mark it as complete.
            //     >
            //     > - [x] Milk
            //     > - [?] Eggs
            //     > - [-] Eggs
            //     "#},
            // ),
            // (
            //     "nesting_lists",
            //     indoc! { r#"## Nesting lists
            //     You can nest any type of list—ordered, unordered, or task lists—under any other type of list.
            //
            //     To create a nested list, indent one or more list items. You can mix list types within a nested structure:
            //
            //     1. First list item
            //        1. Ordered nested list item
            //     2. Second list item
            //        - Unordered nested list item
            //     "#},
            // ),
            // (
            //     "nesting_task_lists",
            //     indoc! { r#"## Nesting task lists
            //     Similarly, you can create a nested task list by indenting one or more list items:
            //
            //     - [ ] Task item 1
            //      - [ ] Subtask 1
            //     - [ ] Task item 2
            //      - [ ] Subtask 1
            //     "#},
            // ),
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
            // (
            //     "code_blocks",
            //     indoc! { r#"## Code blocks
            //     To format code as a block, enclose it with three backticks or three tildes.
            //
            //     ```md
            //     cd ~/Desktop
            //     ```
            //
            //     You can also create a code block by indenting the text using `Tab` or 4 blank spaces.
            //
            //         cd ~/Desktop
            //
            //     "#},
            // ),
            // (
            //     "code_syntax_highlighting_in_blocks",
            //     indoc! { r#"## Code syntax highlighting in blocks
            //     You can add syntax highlighting to a code block, by adding a language code after the first set of backticks.
            //
            //     ```js
            //     function fancyAlert(arg) {
            //       if(arg) {
            //         $.facebox({div:'#foo'})
            //       }
            //     }
            //     ```
            //     "#},
            // ),
        ];

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();

        tests.into_iter().for_each(|(name, content)| {
            let mut state = State::new(content, name, Path::new("test.md"));
            state.set_view(View::Edit(EditMode::Source));

            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    state.resize_viewport(frame.area().as_size());
                    state.cursor_down(4);
                    // state.update_layout();
                    NoteEditor::default().render(frame.area(), frame.buffer_mut(), &mut state)
                })
                .unwrap();
            println!("{:?}", state);

            assert_snapshot!(name, terminal.backend());
        });
    }
}
