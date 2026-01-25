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
        let byte_idx = idx.saturating_sub(self.source_range.start);
        let byte_idx = self.content.floor_char_boundary(byte_idx);

        self.content.insert(byte_idx, c);
        self.source_range.end += c.len_utf8();
        self.modified = true;
    }

    pub fn delete_char(&mut self, idx: usize) -> Option<usize> {
        let byte_idx = idx.saturating_sub(self.source_range.start);

        if let Some((byte_idx, _)) = self.content[..byte_idx].char_indices().next_back() {
            let c = self.content.remove(byte_idx);
            self.source_range.end = self.source_range.end.saturating_sub(c.len_utf8());
            self.modified = true;
            Some(c.len_utf8())
        } else {
            None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_ascii_character() {
        let mut buffer = TextBuffer::new("Hello world", 0..11);
        buffer.insert_char('X', 6);
        assert_eq!(buffer.content, "Hello Xworld");
        assert_eq!(buffer.source_range, 0..12); // +1 byte
        assert!(buffer.modified);
    }

    #[test]
    fn test_insert_unicode_symbol() {
        let mut buffer = TextBuffer::new("Hello world", 0..11);
        buffer.insert_char('✺', 6);
        assert_eq!(buffer.content, "Hello ✺world");
        assert_eq!(buffer.source_range, 0..14); // +3 bytes for emoji
        assert!(buffer.modified);
    }

    #[test]
    fn test_insert_emoji() {
        let mut buffer = TextBuffer::new("Hello world", 0..11);
        buffer.insert_char('😀', 6);
        assert_eq!(buffer.content, "Hello 😀world");
        assert_eq!(buffer.source_range, 0..15); // +4 bytes for emoji
        assert!(buffer.modified);
    }

    #[test]
    fn test_insert_multiple_emojis() {
        let mut buffer = TextBuffer::new("AB", 0..2);
        buffer.insert_char('😀', 1);
        assert_eq!(buffer.content, "A😀B");
        assert_eq!(buffer.source_range, 0..6); // +4 bytes

        buffer.insert_char('😃', 5);
        assert_eq!(buffer.content, "A😀😃B");
        assert_eq!(buffer.source_range, 0..10); // +4 bytes
    }

    #[test]
    fn test_delete_ascii_character() {
        let mut buffer = TextBuffer::new("Hello world", 0..11);
        let deleted_len = buffer.delete_char(6);
        assert_eq!(deleted_len, Some(1));
        assert_eq!(buffer.content, "Helloworld");
        assert_eq!(buffer.source_range, 0..10); // -1 byte
        assert!(buffer.modified);
    }

    #[test]
    fn test_delete_emoji() {
        let mut buffer = TextBuffer::new("Hello 😀world", 0..15);
        let deleted_len = buffer.delete_char(10);
        assert_eq!(deleted_len, Some(4)); // Emoji is 4 bytes
        assert_eq!(buffer.content, "Hello world");
        assert_eq!(buffer.source_range, 0..11); // -4 bytes
        assert!(buffer.modified);
    }

    #[test]
    fn test_delete_at_start() {
        let mut buffer = TextBuffer::new("Hello", 0..5);
        let deleted_len = buffer.delete_char(0);
        assert_eq!(deleted_len, None);
        assert_eq!(buffer.content, "Hello");
        assert_eq!(buffer.source_range, 0..5);
    }

    #[test]
    fn test_insert_with_offset_source_range() {
        let mut buffer = TextBuffer::new("Hello", 10..15);
        buffer.insert_char('X', 13);
        assert_eq!(buffer.content, "HelXlo");
        assert_eq!(buffer.source_range, 10..16); // +1 byte
    }

    #[test]
    fn test_insert_emoji_with_offset() {
        let mut buffer = TextBuffer::new("AB", 20..22);
        buffer.insert_char('😀', 21);
        assert_eq!(buffer.content, "A😀B");
        assert_eq!(buffer.source_range, 20..26); // +4 bytes
    }

    #[test]
    fn test_write_modified_content() {
        let original = "Hello world! This is a test.";
        let mut buffer = TextBuffer::new("world", 6..11);
        buffer.insert_char('😀', 9);
        let new_content = buffer.write(original);
        assert_eq!(new_content, "Hello wor😀ld! This is a test.");
    }
}
