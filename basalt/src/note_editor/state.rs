use core::fmt;

use std::{
    fs::File,
    io::{self, Write},
    ops::RangeBounds,
    path::PathBuf,
    slice::SliceIndex,
};

use ratatui::widgets::ScrollbarState;
use tui_textarea::Input;

use super::{markdown_parser, text_buffer::CursorMove, TextBuffer};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Scrollbar {
    pub state: ScrollbarState,
    pub position: usize,
}

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
pub struct EditorState<'text_buffer> {
    pub view: View,
    pub file_name: String,
    text_buffer: TextBuffer<'text_buffer>,
    content: String,
    content_original: String,
    path: PathBuf,
    nodes: Vec<markdown_parser::Node>,
    scrollbar: Scrollbar,
    pub current_row: usize,
    // TODO: This can be utilized after toast implementation
    // error_message: Option<String>,
    active: bool,
    pub modified: bool,
    dirty: bool,
}

impl<'text_buffer> EditorState<'text_buffer> {
    pub fn content_slice<R>(&self, range: R) -> &str
    where
        R: RangeBounds<usize> + SliceIndex<str, Output = str>,
    {
        &self.content[range]
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn is_editing(&self) -> bool {
        self.view == View::Edit(EditMode::Source)
    }

    pub fn nodes(&self) -> &[markdown_parser::Node] {
        self.nodes.as_slice()
    }

    pub fn nodes_as_mut(&mut self) -> &mut [markdown_parser::Node] {
        self.nodes.as_mut_slice()
    }

    pub fn scrollbar(&self) -> &Scrollbar {
        &self.scrollbar
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn new(file_name: &str, content: &str, path: PathBuf) -> Self {
        Self {
            file_name: file_name.to_string(),
            nodes: markdown_parser::from_str(content),
            content_original: content.to_string(),
            content: content.to_string(),
            path,
            ..Default::default()
        }
    }

    pub fn set_file_name(&mut self, file_name: &str) {
        self.file_name = file_name.to_string();
    }

    pub fn set_content(&mut self, content: &str) {
        self.nodes = markdown_parser::from_str(content);
        self.content_original = content.to_string();
        self.content = content.to_string();
        self.update_text_buffer();
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    pub fn exit_insert(&mut self) {
        self.intermediate_save();
    }

    fn intermediate_save(&mut self) {
        if let Some(node) = self.nodes().get(self.current_row) {
            let start = node.source_range.start;
            let end = node.source_range.end;

            let str_start = &self.content_slice(..start.saturating_sub(1));
            let str_end = &self.content_slice(end..);

            let modified_str = self.text_buffer().to_string();

            let complete_modified_content = [str_start, modified_str.as_str(), str_end].join("\n");

            if self.content != complete_modified_content {
                self.nodes = markdown_parser::from_str(&complete_modified_content);
                self.content = complete_modified_content;
                self.update_text_buffer();
            }

            self.modified = self.content != self.content_original;
        }
    }

    pub fn delete_char(&mut self) {
        let (row, col) = self.text_buffer.cursor();

        if row == 0 && col == 0 && self.text_buffer().to_string().trim().is_empty() {
            self.intermediate_save();
        } else if row == 0 && col == 0 && self.current_row != 0 {
            let current_row = self.current_row;
            let content = self.content.clone();
            let mut nodes = self.nodes_as_mut().to_vec();

            if let Some(current_node_range_end) =
                nodes.get(current_row).map(|node| node.source_range.end)
            {
                if let Some(prev_node) = nodes.get_mut(current_row - 1) {
                    let content = &content[prev_node.source_range.clone()];
                    prev_node.source_range = prev_node.source_range.start..current_node_range_end;
                    self.update_text_buffer_content(content);
                    nodes.remove(current_row);
                    self.nodes = nodes;
                    self.current_row = current_row.saturating_sub(1);
                    self.dirty = true;
                }
            }
        } else {
            self.dirty = true;
            self.text_buffer.edit(Input {
                key: tui_textarea::Key::Backspace,
                ctrl: false,
                alt: false,
                shift: false,
            });
        }
    }

    pub fn edit(&mut self, input: Input) {
        self.text_buffer.edit(input);
        if self.text_buffer.is_modified() {
            self.dirty = true;
        }
    }

    pub fn cursor_up(&mut self) {
        let (row, _) = self.text_buffer.cursor();
        if row == 0 {
            if self.dirty {
                self.intermediate_save();
                self.dirty = false;
            }

            if self.current_row == 0 {
                return;
            }

            self.current_row = self.current_row.saturating_sub(1);
            self.update_text_buffer();
            self.text_buffer.cursor_move(CursorMove::Bottom);
        } else {
            self.text_buffer.cursor_move(CursorMove::Up);
        }
    }

    pub fn cursor_left(&mut self) {
        self.text_buffer.cursor_move(CursorMove::Left);
    }

    pub fn cursor_right(&mut self) {
        self.text_buffer.cursor_move(CursorMove::Right);
    }

    pub fn cursor_move_col(&mut self, cursor_move_col: i32) {
        self.text_buffer.cursor_move((0, cursor_move_col).into());
    }

    pub fn cursor_word_forward(&mut self) {
        self.text_buffer.cursor_move(CursorMove::WordForward);
    }

    pub fn cursor_word_backward(&mut self) {
        self.text_buffer.cursor_move(CursorMove::WordBackward);
    }

    pub fn set_row(&mut self, row: usize) {
        self.current_row = row;
    }

    pub fn cursor_down(&mut self) {
        let (row, _) = self.text_buffer.cursor();
        if row < self.text_buffer.lines().len().saturating_sub(1) {
            self.text_buffer.cursor_move(CursorMove::Down);
        } else {
            if self.dirty {
                self.intermediate_save();
                self.dirty = false;
            }

            let nodes_amount = self.nodes.len();

            if self.current_row == nodes_amount.saturating_sub(1) {
                return;
            }

            self.current_row = self
                .current_row
                .saturating_add(1)
                .min(self.nodes.len().saturating_sub(1));

            self.update_text_buffer();
            self.text_buffer.cursor_move(CursorMove::Top);
        }
    }

    pub fn save(&mut self) {
        if !self.modified {
            return;
        }

        match self.save_modified_to_file() {
            Ok(_) => {}
            Err(_err) => {
                // TODO: Display error messages
                // error_message: Some(format!("Failed to save file: {}", err)),
            }
        }
    }

    fn save_modified_to_file(&mut self) -> io::Result<()> {
        let mut file = File::create(&self.path)?;
        file.write_all(self.content.as_bytes())?;
        self.modified = false;
        Ok(())
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let new_position = self.scrollbar.position.saturating_sub(amount);
        let new_state = self.scrollbar.state.position(new_position);

        // TODO: Advance cursor and try to keep the cursor centered.
        // Look for inspiration from the explorer module list scrolling where the list item is kept
        // in the center, if it is possible. This should be used to scroll the view instead of
        // directly changing the scrollbar in this function.
        self.scrollbar = Scrollbar {
            state: new_state,
            position: new_position,
        }
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let new_position = self.scrollbar.position.saturating_add(amount);
        let new_state = self.scrollbar.state.position(new_position);

        self.scrollbar = Scrollbar {
            state: new_state,
            position: new_position,
        }
    }

    pub fn set_view(&mut self, view: View) {
        self.view = view;
    }

    pub fn text_buffer(&self) -> &TextBuffer<'text_buffer> {
        &self.text_buffer
    }

    pub fn text_buffer_as_mut(&mut self) -> &mut TextBuffer<'text_buffer> {
        self.text_buffer.as_mut()
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn update_text_buffer_content(&mut self, content: &str) {
        let text_buffer_content = self.text_buffer().to_string();
        let (_, col) = self.text_buffer.cursor();
        self.text_buffer = TextBuffer::from(format!("{content}\n{text_buffer_content}"))
            .with_cursor_position((content.lines().count() + 1, col));
    }

    pub fn update_text_buffer(&mut self) {
        if let Some(node) = self.nodes().get(self.current_row) {
            let node_content = self.content_slice(node.source_range.clone());
            self.text_buffer =
                TextBuffer::from(node_content).with_cursor_position(self.text_buffer.cursor());
        }
    }

    pub fn reset(self) -> Self {
        Self {
            view: self.view,
            ..EditorState::default()
        }
    }
}
