use crate::note_editor::ast::SourceRange;

#[derive(Clone, Debug)]
pub struct TextBuffer {
    // FIXME: Change to Rope
    pub content: String,
    pub source_range: SourceRange<usize>,
    pub modified: bool,
    original_source_range: SourceRange<usize>,
}

impl TextBuffer {
    pub fn new(content: &str, source_range: SourceRange<usize>) -> Self {
        Self {
            content: content.to_string(),
            original_source_range: source_range.clone(),
            source_range,
            // FIXME: Implement history to get accurate modified bool
            modified: false,
        }
    }

    pub fn insert_char(&mut self, c: char, idx: usize) {
        let char_idx = idx.saturating_sub(self.source_range.start);

        if let Some((byte_idx, _)) = self.content.char_indices().nth(char_idx) {
            self.content.insert(byte_idx, c);
            self.source_range.end += 1;
            self.modified = true;
        }
    }

    pub fn delete_char(&mut self, idx: usize) {
        let char_idx = idx.saturating_sub(self.source_range.start);
        if let Some((byte_idx, _)) = self.content.char_indices().nth(char_idx.saturating_sub(1)) {
            self.content.remove(byte_idx);
            self.source_range.end = self.source_range.end.saturating_sub(1);
            self.modified = true;
        }
    }

    pub fn write(&self, original_content: &str) -> String {
        if self.modified {
            let str_start = &original_content[..self.original_source_range.start];
            let str_end = &original_content[self.original_source_range.end..];
            format!("{}{}{}", str_start, self.content, str_end)
        } else {
            original_content.to_owned()
        }
    }
}
