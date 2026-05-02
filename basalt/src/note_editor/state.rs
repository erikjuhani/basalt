use std::{
    fmt,
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

use ratatui::layout::Size;

use crate::{
    config::Symbols,
    note_editor::{
        ast::{self},
        cursor::{self, Cursor},
        parser,
        rich_text::RichText,
        text_buffer::TextBuffer,
        viewport::Viewport,
        virtual_document::VirtualDocument,
    },
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
    pub symbols: Symbols,
    filepath: PathBuf,
    filename: String,
    active: bool,
    insert_mode: bool,
    vim_mode: bool,
    editor_enabled: bool,
    modified: bool,
    viewport: Viewport,
    text_buffer: Option<TextBuffer>,
    /// Which block is currently in raw/edit mode. Stored explicitly so
    /// the layout always matches the text_buffer, even when the cursor
    /// position would temporarily resolve to a different block.
    editing_block: Option<usize>,
}

impl<'a> NoteEditorState<'a> {
    pub fn new(content: &str, filename: &str, filepath: &Path, symbols: &Symbols) -> Self {
        let ast_nodes = parser::from_str(content);
        let content = content.to_string();
        Self {
            text_buffer: None,
            content: content.clone(),
            view: View::Read,
            cursor: Cursor::default(),
            viewport: Viewport::default(),
            symbols: symbols.clone(),
            virtual_document: VirtualDocument::new(symbols),
            filename: filename.to_string(),
            filepath: filepath.to_path_buf(),
            ast_nodes,
            active: false,
            insert_mode: false,
            vim_mode: false,
            editor_enabled: false,
            modified: false,
            editing_block: None,
        }
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn is_editing(&self) -> bool {
        matches!(self.view, View::Edit(..))
    }

    pub fn insert_mode(&self) -> bool {
        self.insert_mode
    }

    pub fn set_insert_mode(&mut self, mode: bool) {
        self.insert_mode = mode;
    }

    pub fn vim_mode(&self) -> bool {
        self.vim_mode
    }

    pub fn set_vim_mode(&mut self, mode: bool) {
        self.vim_mode = mode;
    }

    pub fn editor_enabled(&self) -> bool {
        self.editor_enabled
    }

    pub fn set_editor_enabled(&mut self, enabled: bool) {
        self.editor_enabled = enabled;
    }

    pub fn text_buffer(&self) -> Option<&TextBuffer> {
        self.text_buffer.as_ref()
    }

    pub fn enter_insert(&mut self, block_idx: usize) {
        // Commit any pending edits from the previous block before switching.
        self.commit_text_buffer();

        self.editing_block = Some(block_idx);
        if let Some(node) = self.ast_nodes.get(block_idx) {
            let source_range = node.source_range();
            if let Some(content) = self.content.get(source_range.clone()) {
                self.text_buffer = Some(TextBuffer::new(content, source_range.clone()));
            }
        } else if self.content.is_empty() {
            // Only create an empty node for genuinely empty files, not when
            // blocks haven't been laid out yet.
            let empty_node = ast::Node::Paragraph {
                text: RichText::empty(),
                source_range: 0..0,
            };
            self.text_buffer = Some(TextBuffer::new("", empty_node.source_range().clone()));
            self.ast_nodes.push(empty_node);
        }
    }

    pub fn exit_insert(&mut self) {
        if matches!(self.view, View::Read) {
            return;
        }

        self.commit_text_buffer();
        self.text_buffer = None;
        self.editing_block = None;
    }

    /// Write the current text_buffer back to self.content if it was modified,
    /// re-parse AST nodes. Returns Some if content changed.
    pub fn commit_text_buffer(&mut self) -> Option<()> {
        let buffer = self.text_buffer()?;
        if buffer.modified {
            let new_content = buffer.write(&self.content);
            let changed = self.content != new_content;
            self.content = new_content;
            self.ast_nodes = parser::from_str(&self.content);
            self.modified = self.modified || changed;
            Some(())
        } else {
            None
        }
    }

    pub fn set_filename(&mut self, name: &str) {
        self.filename = name.to_string();
    }

    pub fn set_filepath(&mut self, path: &Path) {
        self.filepath = path.to_path_buf();
    }

    pub fn insert_char(&mut self, c: char) {
        if let Some(buffer) = &mut self.text_buffer {
            let source_pos = self.cursor.source_offset();
            buffer.insert_char(c, source_pos);

            // Shift source ranges of all nodes after the insertion point by the character's byte length
            let char_byte_len = c.len_utf8();
            self.shift_source_ranges(source_pos, char_byte_len as isize);

            self.update_layout();

            // Jump cursor to position after the inserted character
            self.cursor.update(
                cursor::Message::Jump(source_pos + char_byte_len),
                self.virtual_document.lines(),
                &self.text_buffer,
            );

            self.ensure_cursor_visible();
        }
    }

    pub fn delete_char(&mut self) -> Option<()> {
        // Comments for my own sanity :)
        // 1. Check if we have a text buffer return if not
        let buffer = self.text_buffer()?;

        // 2. Check if we are at the start of range, which means we need to merge with previous
        //    block
        let at_buffer_start = buffer.source_range.start == self.cursor.source_offset();

        // 3. Get previous and current block idx
        let previous_block_idx = self.previous_block_idx();
        let current_block_idx = self.current_block_idx();

        let should_merge = at_buffer_start && previous_block_idx < current_block_idx;

        if should_merge {
            let prev_start = self.ast_nodes.get(previous_block_idx)?.source_range().start;
            let buffer_start = self.text_buffer.as_ref()?.source_range.start;

            let prefix = self.content.get(prev_start..buffer_start)?;
            // 4. Extend the current text buffer to contain the prev node
            self.text_buffer
                .as_mut()?
                .insert_at_start(prev_start, prefix);
            // 5. Set the previous block as the current editing block
            self.editing_block = Some(previous_block_idx);
            // 6. Delete current ast node
            self.ast_nodes.remove(current_block_idx);
        }

        // 7. Get the buffer again since it might have been updated
        let buffer = self.text_buffer.as_mut()?;

        let source_pos = self.cursor.source_offset();
        let deleted_char_byte_len = buffer.delete_char(source_pos)?;

        // We shift by the negative character byte length to move ranges backwards
        self.shift_source_ranges(source_pos, -(deleted_char_byte_len as isize));

        self.update_layout();

        // Position cursor at where the deleted character was.
        let new_cursor_pos = source_pos.saturating_sub(deleted_char_byte_len);
        self.cursor.update(
            cursor::Message::Jump(new_cursor_pos),
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.ensure_cursor_visible();

        Some(())
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn previous_block_idx(&self) -> usize {
        let prev_line = self.cursor.virtual_row().saturating_sub(1);
        self.virtual_document.line_to_block_idx(prev_line)
    }

    pub fn current_block_idx(&self) -> usize {
        let current_line = self.cursor.virtual_row();
        self.virtual_document.line_to_block_idx(current_line)
    }

    pub fn set_view(&mut self, view: View) {
        let block_idx = self.current_block_idx();

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

            let current_block_idx = self.editing_block;

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
        self.modified || self.text_buffer().is_some_and(|buffer| buffer.modified)
    }

    pub fn cursor_word_forward(&mut self) {
        use cursor::Message::*;

        self.cursor.update(
            MoveWordForward,
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.ensure_cursor_visible();
    }

    pub fn cursor_word_backward(&mut self) {
        use cursor::Message::*;

        self.cursor.update(
            MoveWordBackward,
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.ensure_cursor_visible();
    }

    pub fn cursor_left(&mut self, amount: usize) {
        use cursor::Message::*;

        self.cursor.update(
            MoveLeft(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.ensure_cursor_visible();
    }

    pub fn cursor_right(&mut self, amount: usize) {
        use cursor::Message::*;

        self.cursor.update(
            MoveRight(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.ensure_cursor_visible();
    }

    pub fn cursor_to_end(&mut self) {
        let last_block = self.virtual_document.blocks().len().saturating_sub(1);
        self.cursor_jump(last_block);
        // After jumping to the last block (which lands on its first line),
        // move down to reach the actual last line within that block.
        self.cursor_down(usize::MAX);
    }

    pub fn cursor_jump(&mut self, idx: usize) {
        let prev_block_idx = self.current_block_idx();

        if let Some(block) = self.virtual_document.blocks().get(idx) {
            self.cursor.update(
                cursor::Message::Jump(block.source_range.start),
                self.virtual_document.lines(),
                &self.text_buffer,
            );
        }

        self.relayout_on_block_change(prev_block_idx);
        self.ensure_cursor_visible();
    }

    pub fn update_layout(&mut self) {
        use cursor::Message::*;

        // Deferred initialization: if Edit mode was set before the viewport was
        // sized (e.g. vim_mode at note open), initialize the text buffer now that
        // the virtual document has been laid out.
        //
        // When re-entering insert mode after an exit_insert (e.g. ESC then `i`
        // again in vim mode), the virtual_document still reflects the layout
        // from before commit_text_buffer re-parsed `ast_nodes`, so
        // `current_block_idx` derived from `cursor.virtual_row` would point at
        // a stale block. Instead pick the block whose source range contains
        // the cursor's source offset, so the new buffer starts where the
        // cursor actually is.
        if matches!(self.view, View::Edit(..)) && self.text_buffer.is_none() {
            let offset = self.cursor.source_offset();
            let block_idx = self
                .ast_nodes
                .iter()
                .position(|node| node.source_range().contains(&offset))
                .or_else(|| {
                    self.ast_nodes
                        .iter()
                        .rposition(|node| node.source_range().end <= offset)
                })
                .unwrap_or_else(|| self.current_block_idx());
            self.enter_insert(block_idx);
            self.cursor.update(
                SwitchMode(cursor::CursorMode::Edit),
                self.virtual_document.lines(),
                &self.text_buffer,
            );
        }

        let current_block_idx = self.editing_block;

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
        let prev_block_idx = self.current_block_idx();

        self.cursor.update(
            cursor::Message::MoveUp(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.relayout_on_block_change(prev_block_idx);
        self.ensure_cursor_visible();
    }

    pub fn cursor_down(&mut self, amount: usize) {
        let prev_block_idx = self.current_block_idx();

        self.cursor.update(
            cursor::Message::MoveDown(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.relayout_on_block_change(prev_block_idx);
        self.ensure_cursor_visible();
    }

    /// When the cursor crosses a block boundary in Edit mode, switch the
    /// text_buffer to the new block and re-layout so the new block is
    /// rendered in raw mode.
    ///
    /// After re-layout the source offset from the old layout may not
    /// correspond to the same logical position (e.g. code-block visual
    /// vs raw source ranges differ).  We determine whether the cursor
    /// entered the block from above or below by comparing block indices
    /// and whether the jump crossed more than one block (multi-block
    /// jumps like gg/G always go to the entry edge).
    fn relayout_on_block_change(&mut self, prev_block_idx: usize) {
        if !matches!(self.view, View::Edit(..)) {
            return;
        }

        let target_block_idx = self.current_block_idx();
        if target_block_idx == prev_block_idx {
            return;
        }

        let adjacent = prev_block_idx.abs_diff(target_block_idx) == 1;
        let moved_up = target_block_idx < prev_block_idx;
        let use_end = adjacent && moved_up;

        let target_offset = self.ast_nodes.get(target_block_idx).map(|node| {
            let range = node.source_range();
            if use_end {
                range.end.saturating_sub(1).max(range.start)
            } else {
                range.start
            }
        });

        self.enter_insert(target_block_idx);

        self.virtual_document.layout(
            &self.filename,
            &self.content,
            &self.view,
            self.editing_block,
            &self.ast_nodes,
            self.viewport.area().width.into(),
            self.text_buffer.clone(),
        );

        if let Some(offset) = target_offset {
            self.cursor.update(
                cursor::Message::Jump(offset),
                self.virtual_document.lines(),
                &self.text_buffer,
            );
        }
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
        self.ast_nodes
            .iter_mut()
            .for_each(|node| shift_node(node, offset, shift));
    }
}

/// Shifts source ranges of top-level AST nodes and any nested children.
///
/// This function is a helper function intended to shift the source ranges when editing the
/// document. After exiting the edit mode, the source ranges are calculated by the parser, so
/// we don't have to be precise here.
fn shift_node(node: &mut ast::Node, offset: usize, shift: isize) {
    let shift_value = |v: usize| v.checked_add_signed(shift).unwrap_or(0);
    let range = node.source_range();

    if range.end <= offset {
        return;
    }

    let shifted_range = if range.start > offset {
        shift_value(range.start)..shift_value(range.end)
    } else {
        range.start..shift_value(range.end)
    };
    node.set_source_range(shifted_range);

    if let Some(children) = node.children_as_mut() {
        children
            .iter_mut()
            .for_each(|child| shift_node(child, offset, shift));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Size;
    use std::path::Path;

    fn assert_cursor_visible(state: &NoteEditorState, context: &str) {
        let cursor_row = state.cursor.virtual_row() as i32;
        let top = state.viewport().top() as i32;
        let bottom = state.viewport().bottom() as i32;
        assert!(
            cursor_row >= top && cursor_row < bottom,
            "{context}: cursor row {cursor_row} outside viewport [{top}, {bottom})",
        );
    }

    #[test]
    fn test_viewport_scrolls_with_cursor_in_edit_mode() {
        let content = "# Title\n\nLine 1\n\nLine 2\n\nLine 3\n\nLine 4\n\nLine 5\n";

        let mut state =
            NoteEditorState::new(content, "test", Path::new("test.md"), &Symbols::unicode());
        state.resize_viewport(Size::new(40, 4));

        state.cursor_down(2);
        state.set_view(View::Edit(EditMode::Source));

        state.insert_char('\n');
        state.insert_char('\n');
        state.insert_char('\n');
        state.insert_char('\n');
        assert_cursor_visible(&state, "after insert_char");

        state.cursor_right(20);
        assert_cursor_visible(&state, "after cursor_right");

        state.cursor_left(20);
        assert_cursor_visible(&state, "after cursor_left");

        state.cursor_down(5);
        assert_cursor_visible(&state, "after cursor_down");

        state.cursor_up(5);
        assert_cursor_visible(&state, "after cursor_up");
    }
}
