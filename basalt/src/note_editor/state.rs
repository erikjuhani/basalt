use std::{
    fmt,
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

use ratatui::layout::Size;

use crate::note_editor::{
    ast::{self},
    cursor::{self, Cursor},
    parser,
    text_buffer::TextBuffer,
    viewport::Viewport,
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
pub struct NoteEditorState<'a> {
    // FIXME: Use Rope instead of String for O(log n) instead of O(n).
    pub content: String,
    pub view: View,
    pub cursor: Cursor,
    pub ast_nodes: Vec<ast::Node>,
    pub virtual_document: VirtualDocument<'a>,
    filepath: PathBuf,
    filename: String,
    active: bool,
    modified: bool,
    viewport: Viewport,
    text_buffer: Option<TextBuffer>,
}

impl<'a> NoteEditorState<'a> {
    pub fn new(content: &str, filename: &str, filepath: &Path) -> Self {
        let ast_nodes = parser::from_str(content);
        let content = content.to_string();
        Self {
            text_buffer: None,
            content: content.clone(),
            view: View::Read,
            cursor: Cursor::default(),
            viewport: Viewport::default(),
            virtual_document: VirtualDocument::default(),
            filename: filename.to_string(),
            filepath: filepath.to_path_buf(),
            ast_nodes,
            active: false,
            modified: false,
        }
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn is_editing(&self) -> bool {
        matches!(self.view, View::Edit(..))
    }

    pub fn text_buffer(&self) -> Option<&TextBuffer> {
        self.text_buffer.as_ref()
    }

    // FIXME: if document is empty cannot write as there is no markdown block to write on.
    pub fn enter_insert(&mut self, block_idx: usize) {
        if let Some((_, block)) = self.virtual_document.get_block(block_idx) {
            let source_range = block.source_range();
            if let Some(content) = self.content.get(source_range.clone()) {
                self.text_buffer = Some(TextBuffer::new(content, source_range.clone()));
            }
        } else {
            self.text_buffer = Some(TextBuffer::new("", 0..0));
        }
    }

    pub fn exit_insert(&mut self) {
        if matches!(self.view, View::Read) {
            return;
        }

        if let Some(buffer) = self.text_buffer() {
            let new_content = buffer.write(&self.content);
            if self.content != new_content {
                self.content = new_content;
                self.ast_nodes = parser::from_str(&self.content);
                self.update_layout();
                self.modified = true;
            }
        }

        self.text_buffer = None;
    }

    pub fn insert_char(&mut self, c: char) {
        if let Some(buffer) = &mut self.text_buffer {
            let insertion_offset = self.cursor.source_offset();
            buffer.insert_char(c, insertion_offset);

            // Shift source ranges of all nodes after the insertion point
            self.shift_source_ranges(insertion_offset, 1);

            self.update_layout();
            self.cursor_right(1);
        }
    }

    pub fn delete_char(&mut self) {
        if let Some(buffer) = &mut self.text_buffer {
            if buffer.source_range.start == self.cursor.source_offset() {
                // TODO: Get previous block source range start
                // Get current block source range end
                // Get content with source range
                // Create new text buffer that has merged the previous and current blocks
            } else {
                let deletion_offset = self.cursor.source_offset();
                buffer.delete_char(deletion_offset);

                // We shift by -1 (saturating subtraction) to move ranges backwards
                self.shift_source_ranges(deletion_offset, -1);

                self.update_layout();
                self.cursor_left(1);
            }
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn current_block(&self) -> usize {
        *self
            .virtual_document
            .line_to_block()
            .get(self.cursor.virtual_row())
            .unwrap_or(&0)
    }

    pub fn set_view(&mut self, view: View) {
        let block_idx = self.current_block();

        self.view = view;

        use cursor::Message::*;

        match self.view {
            View::Read => {
                self.exit_insert();
                self.update_layout();
                self.cursor.update(
                    SwitchMode(cursor::CursorMode::Read),
                    self.virtual_document.lines(),
                    &None,
                );
            }
            View::Edit(..) => {
                self.enter_insert(block_idx);
                self.update_layout();
                self.cursor.update(
                    SwitchMode(cursor::CursorMode::Edit),
                    self.virtual_document.lines(),
                    &self.text_buffer,
                );
            }
        }
    }

    pub fn resize_viewport(&mut self, size: Size) {
        if self.viewport.size_changed(size) {
            use cursor::Message::*;

            let current_block_idx =
                matches!(self.view, View::Edit(..)).then_some(self.current_block());

            self.virtual_document.layout(
                &self.filename,
                &self.content,
                &self.view,
                current_block_idx,
                &self.ast_nodes,
                size.width.into(),
                self.text_buffer.clone(),
            );

            self.viewport.resize(size);

            self.cursor.update(
                Jump(self.cursor.source_offset()),
                self.virtual_document.lines(),
                &self.text_buffer,
            );

            self.ensure_cursor_visible();
        }
    }

    /// Ensures the cursor is visible within the viewport by scrolling if necessary.
    /// This method should be called after any operation that might cause the cursor
    /// to move outside the visible area (e.g., resize, cursor movement).
    fn ensure_cursor_visible(&mut self) {
        let cursor_row = self.cursor.virtual_row() as i32;
        let viewport_top = self.viewport.top() as i32;
        let viewport_bottom = self.viewport.bottom() as i32;
        let meta_len = self.virtual_document.meta().len() as i32;

        let effective_bottom = viewport_bottom.saturating_sub(meta_len);

        if cursor_row < viewport_top {
            let scroll_offset = cursor_row - viewport_top;
            self.viewport.scroll_by((scroll_offset, 0));
        } else if cursor_row >= effective_bottom {
            let scroll_offset = cursor_row - effective_bottom + 1;
            self.viewport.scroll_by((scroll_offset, 0));
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn modified(&self) -> bool {
        self.text_buffer()
            .map(|buffer| buffer.modified)
            .unwrap_or(self.modified)
    }

    pub fn cursor_word_forward(&mut self) {
        use cursor::Message::*;

        self.cursor.update(
            MoveWordForward,
            self.virtual_document.lines(),
            &self.text_buffer,
        );
    }

    pub fn cursor_word_backward(&mut self) {
        use cursor::Message::*;

        self.cursor.update(
            MoveWordBackward,
            self.virtual_document.lines(),
            &self.text_buffer,
        );
    }

    pub fn cursor_left(&mut self, amount: usize) {
        use cursor::Message::*;

        self.cursor.update(
            MoveLeft(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );
    }

    pub fn cursor_right(&mut self, amount: usize) {
        use cursor::Message::*;

        self.cursor.update(
            MoveRight(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );
    }

    pub fn cursor_jump(&mut self, idx: usize) {
        use cursor::Message::*;

        if let Some(block) = self.virtual_document.blocks().get(idx) {
            self.cursor.update(
                Jump(block.source_range.start),
                self.virtual_document.lines(),
                &self.text_buffer,
            );
        }

        self.ensure_cursor_visible();
    }

    pub fn update_layout(&mut self) {
        use cursor::Message::*;

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
            self.viewport.area().width.into(),
            self.text_buffer.clone(),
        );

        self.cursor.update(
            Jump(self.cursor.source_offset()),
            self.virtual_document.lines(),
            &self.text_buffer,
        );
    }

    pub fn cursor_up(&mut self, amount: usize) {
        use cursor::Message::*;

        let prev_block_idx = self.current_block();

        self.cursor.update(
            MoveUp(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        if matches!(self.view, View::Edit(..)) {
            let current_block_idx = self.current_block();

            if current_block_idx != prev_block_idx {
                self.enter_insert(current_block_idx);

                self.virtual_document.layout(
                    &self.filename,
                    &self.content,
                    &self.view,
                    Some(current_block_idx),
                    &self.ast_nodes,
                    self.viewport.area().width.into(),
                    self.text_buffer.clone(),
                );

                // Recalculate cursor position after layout change
                // The virtual line indices have shifted, so we need to find the new position
                // based on the source offset
                self.cursor.update(
                    Jump(self.cursor.source_offset()),
                    self.virtual_document.lines(),
                    &self.text_buffer,
                );
            }
        }

        self.ensure_cursor_visible();
    }

    pub fn cursor_down(&mut self, amount: usize) {
        use cursor::Message::*;

        let prev_block_idx = self.current_block();

        self.cursor.update(
            MoveDown(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        if matches!(self.view, View::Edit(..)) {
            let current_block_idx = self.current_block();

            if current_block_idx != prev_block_idx {
                self.enter_insert(current_block_idx);

                self.virtual_document.layout(
                    &self.filename,
                    &self.content,
                    &self.view,
                    Some(current_block_idx),
                    &self.ast_nodes,
                    self.viewport.area().width.into(),
                    self.text_buffer.clone(),
                );

                // Recalculate cursor position after layout change
                // The virtual line indices have shifted, so we need to find the new position
                // based on the source offset
                self.cursor.update(
                    Jump(self.cursor.source_offset()),
                    self.virtual_document.lines(),
                    &self.text_buffer,
                );
            }
        }

        self.ensure_cursor_visible();
    }

    pub fn save_to_file(&mut self) -> io::Result<()> {
        if self.modified() {
            let mut file = File::create(&self.filepath)?;
            file.write_all(self.content.as_bytes())?;
            self.modified = false;
        }
        Ok(())
    }

    /// The shift amount can be positive (insertion) or negative (deletion).
    fn shift_source_ranges(&mut self, offset: usize, shift: isize) {
        self.shift_nodes(offset, shift);
    }

    /// Shifts source ranges of top-level AST nodes.
    ///
    /// This function is a helper function intended to shift the source ranges when editing the
    /// document. After exiting the edit mode, the source ranges are calculated by the parser, so
    /// we don't have to be precise here.
    fn shift_nodes(&mut self, offset: usize, shift: isize) {
        let shift_value = |v: usize| v.checked_add_signed(shift).unwrap_or(0);

        // We only take the current node and the rest after it
        let nodes = self
            .ast_nodes
            .iter_mut()
            .filter(|node| node.source_range().end > offset);

        nodes.for_each(|node| {
            let range = node.source_range();
            let shifted_range = if range.start > offset {
                shift_value(range.start)..shift_value(range.end)
            } else {
                range.start..shift_value(range.end)
            };
            node.set_source_range(shifted_range);
        });
    }
}
