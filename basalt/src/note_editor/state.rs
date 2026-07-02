use std::{
    borrow::Cow,
    fmt,
    fs::File,
    io::{self, Write},
    ops::Range,
    path::{Path, PathBuf},
    time::{Duration, Instant},
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionMode {
    Char,
    Line,
}

/// `anchor` is the source byte offset where selection began; the moving end is
/// the cursor's current source offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Selection {
    pub anchor: usize,
    pub mode: SelectionMode,
}

const YANK_FLASH_DURATION: Duration = Duration::from_millis(150);

#[derive(Clone, Debug)]
struct YankFlash {
    range: Range<usize>,
    started: Instant,
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
    selection: Option<Selection>,
    yank_flash: Option<YankFlash>,
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
            selection: None,
            yank_flash: None,
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

            // Move the cursor to the inserted character before laying out so the
            // active (raw) line tracks the cursor's new position.
            self.cursor.update(
                cursor::Message::Jump(source_pos + char_byte_len),
                self.virtual_document.lines(),
                &self.text_buffer,
            );

            self.update_layout();

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

        // The removed byte sits just before the cursor; shift from there (not the
        // cursor) so a node boundary ending at the cursor shrinks with it.
        let deleted_at = source_pos.saturating_sub(deleted_char_byte_len);
        self.shift_source_ranges(deleted_at, -(deleted_char_byte_len as isize));

        // Position the cursor where the deleted character was before laying out
        // so the active (raw) line tracks the cursor's new position.
        self.cursor.update(
            cursor::Message::Jump(deleted_at),
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.update_layout();

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
                // Read mode never pans horizontally; reset so its width-sized
                // fills (code backgrounds, rules) line up with the viewport.
                self.ensure_cursor_visible();
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
                self.cursor.source_offset(),
                &self.ast_nodes,
                size.width.into(),
                self.viewport.left().into(),
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
        let meta_len = self.virtual_document.meta().len() as i32;
        let cursor_row = self.cursor.virtual_row() as i32 + meta_len;
        let cursor_column = self.cursor.virtual_column() as i32;

        let vertical = if cursor_row < self.viewport.top() as i32 {
            cursor_row - self.viewport.top() as i32
        } else if cursor_row >= self.viewport.bottom() as i32 {
            cursor_row - self.viewport.bottom() as i32 + 1
        } else {
            0
        };

        let horizontal = if cursor_column < self.viewport.left() as i32 {
            cursor_column - self.viewport.left() as i32
        } else if cursor_column >= self.viewport.right() as i32 {
            cursor_column - self.viewport.right() as i32 + 1
        } else {
            0
        };

        if (vertical, horizontal) != (0, 0) {
            self.viewport.scroll_by((vertical, horizontal));
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn modified(&self) -> bool {
        self.modified || self.text_buffer().is_some_and(|buffer| buffer.modified)
    }

    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    pub fn is_selecting(&self) -> bool {
        self.selection.is_some()
    }

    /// Enters visual selection in `mode`, or exits if that mode is already active.
    pub fn toggle_selection(&mut self, mode: SelectionMode) {
        self.selection = match self.selection {
            Some(selection) if selection.mode == mode => None,
            _ => Some(Selection {
                anchor: self.cursor.source_offset(),
                mode,
            }),
        };
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Source content as currently displayed, accounting for unsaved edits.
    fn live_content(&self) -> Cow<'_, str> {
        self.text_buffer
            .as_ref()
            .filter(|buffer| buffer.modified)
            .map(|buffer| Cow::Owned(buffer.write(&self.content)))
            .unwrap_or(Cow::Borrowed(&self.content))
    }

    /// Source byte range from anchor to cursor. Charwise includes the character
    /// under the cursor; linewise rounds out to whole lines.
    pub fn selection_range(&self) -> Option<Range<usize>> {
        let selection = self.selection?;
        let content = self.live_content();
        let cursor = self.cursor.source_offset().min(content.len());
        let anchor = selection.anchor.min(content.len());
        let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));

        let range = match selection.mode {
            SelectionMode::Char => {
                let end = hi + content[hi..].chars().next().map_or(0, char::len_utf8);
                lo..end
            }
            SelectionMode::Line => {
                let start = content[..lo].rfind('\n').map_or(0, |i| i + 1);
                let end = content[hi..]
                    .find('\n')
                    .map_or(content.len(), |i| hi + i + 1);
                start..end
            }
        };

        Some(range)
    }

    pub fn selected_text(&self) -> Option<String> {
        let range = self.selection_range()?;
        self.live_content().get(range).map(str::to_string)
    }

    /// Flashes `range` to acknowledge a yank. The highlight fades on its own
    /// after [`YANK_FLASH_DURATION`].
    pub fn flash_yank(&mut self, range: Range<usize>) {
        self.yank_flash = Some(YankFlash {
            range,
            started: Instant::now(),
        });
    }

    /// The range to flash right now, or `None` once the flash has elapsed.
    pub fn yank_flash_range(&self) -> Option<Range<usize>> {
        self.yank_flash
            .as_ref()
            .filter(|flash| flash.started.elapsed() < YANK_FLASH_DURATION)
            .map(|flash| flash.range.clone())
    }

    pub fn cursor_word_forward(&mut self) {
        use cursor::Message::*;

        let prev_block_idx = self.current_block_idx();
        self.cursor.update(
            MoveWordForward,
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.relayout_on_block_change(prev_block_idx);
        self.ensure_cursor_visible();
    }

    pub fn cursor_word_backward(&mut self) {
        use cursor::Message::*;

        let prev_block_idx = self.current_block_idx();
        self.cursor.update(
            MoveWordBackward,
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.relayout_on_block_change(prev_block_idx);
        self.ensure_cursor_visible();
    }

    pub fn cursor_left(&mut self, amount: usize) {
        use cursor::Message::*;

        let prev_block_idx = self.current_block_idx();
        self.cursor.update(
            MoveLeft(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.relayout_on_block_change(prev_block_idx);
        self.ensure_cursor_visible();
    }

    pub fn cursor_right(&mut self, amount: usize) {
        use cursor::Message::*;

        let prev_block_idx = self.current_block_idx();
        self.cursor.update(
            MoveRight(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );

        self.relayout_on_block_change(prev_block_idx);
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
            self.cursor.source_offset(),
            &self.ast_nodes,
            self.viewport.area().width.into(),
            self.viewport.left().into(),
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
        let prev_row = self.cursor.virtual_row();

        self.cursor.update(
            cursor::Message::MoveUp(amount),
            self.virtual_document.lines(),
            &self.text_buffer,
        );
        let consumed = prev_row.saturating_sub(self.cursor.virtual_row());
        self.viewport.scroll_up(amount.saturating_sub(consumed));

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

    /// Re-layout while editing so the raw (source) line tracks the cursor.
    ///
    /// Within a block only the cursor's line is shown raw, so any move that
    /// lands on a different line re-runs the layout. Crossing a block boundary
    /// additionally switches the text_buffer to the new block.
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
            self.update_layout();
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
            target_offset.unwrap_or_else(|| self.cursor.source_offset()),
            &self.ast_nodes,
            self.viewport.area().width.into(),
            self.viewport.left().into(),
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
        let meta_len = state.virtual_document.meta().len() as i32;
        let cursor_screen_row = state.cursor.virtual_row() as i32 + meta_len;
        let top = state.viewport().top() as i32;
        let bottom = state.viewport().bottom() as i32;
        assert!(
            cursor_screen_row >= top && cursor_screen_row < bottom,
            "{context}: cursor screen row {cursor_screen_row} outside viewport [{top}, {bottom})",
        );

        let cursor_column = state.cursor.virtual_column() as i32;
        let left = state.viewport().left() as i32;
        let right = state.viewport().right() as i32;
        assert!(
            cursor_column >= left && cursor_column < right,
            "{context}: cursor column {cursor_column} outside viewport [{left}, {right})",
        );
    }

    fn line_texts(state: &NoteEditorState) -> Vec<String> {
        state
            .virtual_document
            .lines()
            .iter()
            .map(|line| {
                line.clone()
                    .spans()
                    .iter()
                    .map(|span| span.content.to_string())
                    .collect()
            })
            .collect()
    }

    /// Editing a block quote shows raw `>` markers line by line: the cursor's
    /// line is raw, the rest keep the rendered `┃` marker (read mode renders all
    /// lines with `┃`). Ref: issue #486.
    #[test]
    fn test_block_quote_raw_marker_line_by_line() {
        let mut state = NoteEditorState::new(
            "> quote line one\n> quote line two\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(50, 14));

        let read = line_texts(&state);
        assert!(
            read.iter().any(|line| line.contains("┃ quote line one")),
            "read mode renders the quote marker, got {read:?}",
        );

        state.set_view(View::Edit(EditMode::Source));
        let edit = line_texts(&state);
        // Cursor on the first line: it is raw, the second stays rendered.
        assert!(
            edit.iter().any(|line| line.contains("> quote line one")),
            "cursor line shows the raw marker, got {edit:?}",
        );
        assert!(
            edit.iter().any(|line| line.contains("┃ quote line two")),
            "non-cursor line keeps the rendered marker, got {edit:?}",
        );

        // Move to the second line: now it is the raw one.
        state.cursor_down(1);
        let edit = line_texts(&state);
        assert!(
            edit.iter().any(|line| line.contains("┃ quote line one")),
            "first line now rendered, got {edit:?}",
        );
        assert!(
            edit.iter().any(|line| line.contains("> quote line two")),
            "second line now raw, got {edit:?}",
        );
    }

    /// The block-quote markers are coloured even on the raw cursor line — and
    /// every nesting level, not just the first. Ref: #486.
    #[test]
    fn test_block_quote_cursor_marker_is_colored() {
        use ratatui::style::Color;

        let mut state = NoteEditorState::new(
            "> > deep\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(40, 8));
        state.set_view(View::Edit(EditMode::Source)); // cursor on the quote line

        let markers: Vec<_> = state
            .virtual_document
            .lines()
            .iter()
            .flat_map(|line| line.clone().spans())
            .filter(|span| span.content.contains('>'))
            .collect();
        assert!(!markers.is_empty(), "quote markers should be present");
        assert!(
            markers
                .iter()
                .all(|span| span.style.fg == Some(Color::Magenta)),
            "every `>` marker (all nesting levels) should be coloured",
        );
    }

    /// Lines inside a fenced code block are rendered literally while editing —
    /// never decorated as markdown (list bullets, headings, etc.). Ref: #486.
    #[test]
    fn test_code_block_content_not_decorated_when_editing() {
        let mut state = NoteEditorState::new(
            "```\n- not a list\n# not a heading\n```\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(40, 10));
        state.set_view(View::Edit(EditMode::Source));

        let lines = line_texts(&state);
        assert!(
            lines.iter().any(|line| line.contains("- not a list")),
            "code content stays literal, got {lines:?}",
        );
        assert!(
            !lines.iter().any(|line| line.contains('●')),
            "code content must not get a list bullet, got {lines:?}",
        );
        assert!(
            lines.iter().any(|line| line.contains("# not a heading")),
            "code content keeps its `#`, got {lines:?}",
        );
    }

    /// A heading keeps its rendered style and underline while editing; the `#`
    /// markers stay visible (and editable). Ref: issue #486.
    #[test]
    fn test_heading_keeps_underline_when_editing() {
        let mut state = NoteEditorState::new(
            "## Title\n\npara\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(40, 10));
        state.set_view(View::Edit(EditMode::Source)); // cursor on the heading

        let lines = line_texts(&state);
        assert!(
            lines.iter().any(|line| line.contains("## Title")),
            "heading markers stay visible, got {lines:?}",
        );
        assert!(
            lines
                .iter()
                .any(|line| !line.is_empty() && line.chars().all(|c| c == '─')),
            "heading keeps its underline, got {lines:?}",
        );
    }

    /// When the active block's range swallows the blank lines before the next
    /// block (a list does this), those blanks must still render as editable
    /// lines instead of vanishing. Ref: issue #486.
    #[test]
    fn test_multiple_blank_lines_after_active_block_render() {
        // The list's source range includes the three trailing blank lines.
        let mut state = NoteEditorState::new(
            "- item one\n\n\n\npara2\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(50, 14));
        state.set_view(View::Edit(EditMode::Source));

        let lines = line_texts(&state);
        let item = lines
            .iter()
            .position(|line| line.contains("item one"))
            .unwrap();
        let para = lines
            .iter()
            .position(|line| line.contains("para2"))
            .unwrap();
        assert_eq!(
            para - item,
            4,
            "expected three blank lines between the list and the paragraph, got {lines:?}",
        );
    }

    /// Merging a block into the previous one (delete at the buffer start) must
    /// keep the merged text visible — the active block renders from a fresh
    /// parse of the buffer, not the stale (pre-merge) AST. Ref: issue #486.
    #[test]
    fn test_merge_into_previous_block_keeps_text() {
        let mut state = NoteEditorState::new(
            "- item one\n\nsecond paragraph\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(50, 12));
        state.set_view(View::Edit(EditMode::Source));

        // Down to the blank, then onto "second paragraph", then merge it up.
        state.cursor_down(1);
        state.cursor_down(1);
        assert_eq!(state.cursor.source_offset(), 12);
        state.delete_char();

        let lines = line_texts(&state);
        assert!(
            lines.iter().any(|line| line.contains("second paragraph")),
            "merged text must stay visible, got {lines:?}",
        );
        state.commit_text_buffer();
        assert_eq!(state.content, "- item one\nsecond paragraph\n");
    }

    /// An empty list item (`- ` with no text) must still render its marker row
    /// instead of vanishing — otherwise splitting an item or adding a line looks
    /// like nothing happened. Ref: issue #486.
    #[test]
    fn test_empty_list_item_renders_marker() {
        let mut state = NoteEditorState::new(
            "- one\n- \n- three\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(40, 12));
        state.set_view(View::Edit(EditMode::Source));

        // Cursor on "- one"; the empty middle item still occupies a row.
        let lines = line_texts(&state);
        assert!(
            lines.iter().any(|line| line == "● "),
            "empty item must render its marker, got {lines:?}",
        );
        assert!(lines.iter().any(|line| line.contains("three")), "{lines:?}");
    }

    /// On a tab-indented line the cursor column must line up with the displayed
    /// (tab-expanded) text: a tab is one byte but two columns. Ref: issue #486.
    #[test]
    fn test_cursor_column_aligns_on_tab_indented_line() {
        let mut state = NoteEditorState::new(
            "- a\n\t- b\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(40, 12));
        state.set_view(View::Edit(EditMode::Source));
        state.cursor_down(1); // onto "\t- b" -> raw "  - b", cursor on 'b'

        // 'b' is byte offset 7; the tab (1 byte) renders as 2 columns, so 'b'
        // sits at display column 4, not 3.
        assert_eq!(state.cursor.source_offset(), 7);
        assert_eq!(state.cursor.virtual_column(), 4);

        // Stepping left onto the marker stays aligned with the display.
        state.cursor_left(2);
        assert_eq!(state.cursor.source_offset(), 5); // the '-'
        assert_eq!(state.cursor.virtual_column(), 2); // after the 2-col tab
    }

    /// A tab-indented nested list keeps its indentation when the list is the
    /// active (editing) block — raw tabs are expanded to spaces so the terminal
    /// doesn't collapse them. Ref: issue #486.
    #[test]
    fn test_tab_indented_nested_list_keeps_indentation_when_editing() {
        let mut state = NoteEditorState::new(
            "1. one\n2. two\n\t- nested\n\t\t- deep\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(50, 12));
        state.set_view(View::Edit(EditMode::Source));
        // Onto the list (cursor on "1. one"); the nested items render decorated.
        let lines = line_texts(&state);
        assert!(lines.iter().any(|line| line == "  ○ nested"), "{lines:?}");
        assert!(lines.iter().any(|line| line == "    ◆ deep"), "{lines:?}");
    }

    /// A nested list renders cleanly while editing: the cursor's line is raw,
    /// deeper items keep their indentation and rendered bullet, and no spurious
    /// indent-only lines appear. Ref: issue #486.
    #[test]
    fn test_nested_list_renders_cleanly_when_editing() {
        let mut state = NoteEditorState::new(
            "- item one\n  - nested one\n  - nested two\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(50, 12));
        state.set_view(View::Edit(EditMode::Source));

        let lines = line_texts(&state);
        // Cursor on item one: raw marker.
        assert_eq!(lines.first().map(String::as_str), Some("- item one"));
        // Nested items: indentation preserved, rendered bullet.
        assert!(
            lines.iter().any(|line| line == "  ○ nested one"),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|line| line == "  ○ nested two"),
            "{lines:?}"
        );
        // No stray indent-only line (the bug this redesign fixes).
        assert!(
            !lines
                .iter()
                .any(|line| !line.is_empty() && line.trim().is_empty()),
            "spurious whitespace line: {lines:?}",
        );
    }

    /// Deleting the blank line before a list item must pull the whole item up
    /// intact — its marker and text stay on one row. Ref: issue #486.
    #[test]
    fn test_delete_blank_before_item_keeps_item_intact() {
        let mut state = NoteEditorState::new(
            "- first\n\n- second item text\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(50, 12));
        state.set_view(View::Edit(EditMode::Source));

        // Onto the blank line between the items, then delete it.
        state.cursor_down(1);
        state.delete_char();

        let lines = line_texts(&state);
        assert!(
            lines.iter().any(|line| line.contains("second item text")),
            "item must stay intact on one row, got {lines:?}",
        );
        state.commit_text_buffer();
        assert_eq!(state.content, "- first\n- second item text\n");
    }

    /// A loose list (blank line between items) must render a single blank line
    /// between items even when the cursor is on an item rendered raw — the raw
    /// item's trailing blank and the list's empty-line preservation must not
    /// stack. The blank must stay editable (the cursor can land on it). Ref:
    /// issue #486.
    #[test]
    fn test_loose_list_blank_not_doubled_when_item_raw() {
        let mut state = NoteEditorState::new(
            "- a\n\n- b\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(40, 12));
        state.set_view(View::Edit(EditMode::Source));

        // Cursor lands on item "a", which renders raw.
        let lines = line_texts(&state);
        let a = lines.iter().position(|line| line.contains("- a")).unwrap();
        let b = lines.iter().position(|line| line.contains("b")).unwrap();
        assert_eq!(
            b - a,
            2,
            "expected exactly one blank line between items, got {lines:?}",
        );

        // The blank between the items is reachable, so it can be edited away.
        state.cursor_down(1);
        assert_eq!(
            state.cursor.source_offset(),
            4,
            "cursor should land on the blank line between the items",
        );
        state.delete_char();
        state.commit_text_buffer();
        assert_eq!(state.content, "- a\n- b\n");
    }

    /// Pressing Enter at the very start of a list item must insert a blank line
    /// and push the item down, not drop the item's text. Ref: issue #486.
    #[test]
    fn test_newline_at_start_of_list_item_preserves_text() {
        let mut state = NoteEditorState::new(
            "- hello world\n",
            "test",
            Path::new("test.md"),
            &Symbols::unicode(),
        );
        state.resize_viewport(Size::new(40, 10));
        state.set_view(View::Edit(EditMode::Source));

        state.insert_char('\n');

        let lines = line_texts(&state);
        assert!(
            lines.iter().any(|line| line.contains("- hello world")),
            "item text must survive the newline, got {lines:?}",
        );
        // Cursor lands on the item, now on the second line, at its start.
        assert_eq!(state.cursor.source_offset(), 1);
        assert_eq!(state.cursor.virtual_row(), 1);
        assert_eq!(state.cursor.virtual_column(), 0);
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

    #[test]
    fn test_viewport_scrolls_horizontally_on_long_code_line() {
        // Code-block lines are not wrapped, so a long one overflows the viewport
        // and the cursor must pan it horizontally to stay visible.
        let long = "x".repeat(100);
        let content = format!("```\n{long}\n```\n");

        let mut state =
            NoteEditorState::new(&content, "test", Path::new("test.md"), &Symbols::unicode());
        state.resize_viewport(Size::new(20, 10));
        state.set_view(View::Edit(EditMode::Source));

        // Land on the code line and walk to its end.
        state.cursor_down(1);
        assert_eq!(state.viewport().left(), 0, "no scroll at line start");

        state.cursor_right(80);
        let panned = state.viewport().left();
        assert!(panned > 0, "viewport should pan right to follow the cursor");
        assert_cursor_visible(&state, "after cursor_right on long line");

        state.cursor_left(80);
        assert!(
            state.viewport().left() < panned,
            "viewport should pan back toward the start",
        );
        assert_cursor_visible(&state, "after cursor_left on long line");
    }

    fn widest_line_in_range(state: &NoteEditorState, range: std::ops::Range<usize>) -> usize {
        state
            .virtual_document
            .lines()
            .iter()
            .filter(|line| {
                line.source_range()
                    .is_some_and(|r| range.contains(&r.start))
            })
            .map(|line| line.virtual_spans().iter().map(|span| span.width()).sum())
            .max()
            .expect("a line in the given source range")
    }

    #[test]
    fn test_code_background_extends_past_horizontal_scroll() {
        // The active code block has a short line and a long one. Panning right to
        // follow the long line must keep the short line's background reaching the
        // viewport's right edge (left + width), not stop at its own content.
        let content = format!("```\nshort\n{}\n```\n", "x".repeat(100));

        let mut state =
            NoteEditorState::new(&content, "", Path::new("test.md"), &Symbols::unicode());
        let width = 20;
        state.resize_viewport(Size::new(width, 10));
        state.set_view(View::Edit(EditMode::Source));

        // Step onto the long line and pan into it.
        state.cursor_down(2);
        state.cursor_right(80);
        state.update_layout(); // editor.rs re-lays out against the scroll each frame.

        let left = state.viewport().left() as usize;
        assert!(left > 0, "expected a horizontal scroll");

        let short_line_width = widest_line_in_range(&state, 4..10);
        assert!(
            short_line_width >= left + width as usize,
            "code background ({short_line_width}) must cover the viewport \
             ({left} + {width})",
        );
    }

    #[test]
    fn test_non_active_block_fills_extend_past_horizontal_scroll() {
        // Editing a long line in one block pans the whole viewport. The fills of
        // the *other* visible blocks must extend too, even though those blocks
        // are not the one being edited. Here a second, non-active code block.
        let long = "x".repeat(100);
        let content = format!("```\n{long}\n```\n\n```\nbbb\n```\n");
        // The second code block opens at the fence after the blank line.
        let second_block_start = content.find("\n\n").unwrap() + 2;

        let mut state =
            NoteEditorState::new(&content, "", Path::new("test.md"), &Symbols::unicode());
        let width = 20;
        state.resize_viewport(Size::new(width, 10));
        state.set_view(View::Edit(EditMode::Source));

        // Pan into the long line of the first (active) code block.
        state.cursor_down(1);
        state.cursor_right(80);
        state.update_layout();

        let left = state.viewport().left() as usize;
        assert!(left > 0, "expected a horizontal scroll");

        let non_active_width = widest_line_in_range(&state, second_block_start..content.len());
        assert!(
            non_active_width >= left + width as usize,
            "non-active code background ({non_active_width}) must cover the \
             viewport ({left} + {width})",
        );
    }

    fn edit_state(content: &str) -> NoteEditorState<'static> {
        let mut state =
            NoteEditorState::new(content, "test", Path::new("test.md"), &Symbols::unicode());
        state.resize_viewport(Size::new(40, 10));
        state.set_view(View::Edit(EditMode::Source));
        state
    }

    #[test]
    fn test_charwise_selection_is_inclusive() {
        let mut state = edit_state("hello world\n");

        state.toggle_selection(SelectionMode::Char);
        state.cursor_right(4);

        assert_eq!(state.selected_text().as_deref(), Some("hello"));
    }

    #[test]
    fn test_charwise_selection_extends_backwards() {
        let mut state = edit_state("hello world\n");

        state.cursor_right(4);
        state.toggle_selection(SelectionMode::Char);
        state.cursor_left(4);

        assert_eq!(state.selected_text().as_deref(), Some("hello"));
    }

    #[test]
    fn test_linewise_selection_covers_whole_line() {
        let mut state = edit_state("line one\nline two\n");

        state.cursor_right(3);
        state.toggle_selection(SelectionMode::Line);

        assert_eq!(state.selected_text().as_deref(), Some("line one\n"));
    }

    #[test]
    fn test_toggle_same_mode_clears_selection() {
        let mut state = edit_state("hello\n");

        state.toggle_selection(SelectionMode::Char);
        assert!(state.is_selecting());

        state.toggle_selection(SelectionMode::Char);
        assert!(!state.is_selecting());
        assert_eq!(state.selection_range(), None);
    }
}
