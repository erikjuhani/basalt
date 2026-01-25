use std::ops::ControlFlow;

use ratatui::{
    buffer::Buffer,
    layout::{Offset, Rect},
    style::Style,
    widgets::StatefulWidget,
};
use unicode_width::UnicodeWidthChar;

use crate::note_editor::{text_buffer::TextBuffer, virtual_document::VirtualLine};

#[derive(Clone, Debug)]
pub enum Message {
    // MoveTop,
    // MoveBottom,
    MoveWordForward,
    MoveWordBackward,
    MoveUp(usize),
    MoveDown(usize),
    MoveLeft(usize),
    MoveRight(usize),
    // Move(i32, i32),
    /// Jump the cursor according to the given byte intex
    Jump(usize),
    SwitchMode(CursorMode),
}

#[derive(Clone, Debug, Default)]
pub enum CursorMode {
    #[default]
    Read,
    Edit,
}

#[derive(Clone, Debug, Default)]
pub struct Cursor {
    mode: CursorMode,
    pub source_offset: usize,
    pub virtual_row: usize,
    pub virtual_column: usize,
}

pub fn virtual_position_to_source_offset<'a>(
    (row, col): (usize, usize),
    lines: &'a [VirtualLine<'a>],
) -> Option<usize> {
    let line = lines.get(row).filter(|line| line.has_content())?;
    let source_range = line.source_range()?;
    let mut cur_col = 0;
    // track span width (0 for empty content lines)
    let mut content_col = 0;

    for span in line.virtual_spans() {
        match span.source_range() {
            Some(range) => {
                for (byte_idx, ch) in span.char_indices() {
                    if cur_col >= col {
                        return Some(range.start + byte_idx);
                    }

                    let char_width = ch.width().unwrap_or(0);
                    cur_col += char_width;
                    content_col += char_width;
                }
            }
            _ => cur_col += span.width(),
        }
    }

    // If we've processed all spans and still haven't reached the target column,
    // we're at the end of the line. For empty lines (no content characters) or
    // when col is 0, use start. Otherwise use end to position cursor after the
    // last character.
    if col == 0 || content_col == 0 {
        return Some(source_range.start);
    }
    Some(source_range.end)
}

pub fn source_offset_to_virtual_line<'a>(
    offset: usize,
    lines: &'a [VirtualLine<'a>],
) -> Option<(usize, &'a VirtualLine<'a>)> {
    let virtual_line = lines
        .iter()
        .enumerate()
        .try_fold(None, |fallback, (idx, line)| {
            match line.source_range() {
                // We are inside the line so we can circuit break
                Some(range) if range.contains(&offset) => ControlFlow::Break(Some((idx, line))),
                // We are at the end, but since we have to consider new lines in the source offset we
                // continue to the next line which might match. Otherwise we will use this match as a
                // fallback line.
                Some(range) if range.end == offset => ControlFlow::Continue(Some((idx, line))),
                _ => ControlFlow::Continue(fallback),
            }
        });

    // FIXME: Use into_value when it becomes a stable feature:
    // https://doc.rust-lang.org/std/ops/enum.ControlFlow.html#method.into_value
    match virtual_line {
        ControlFlow::Break(line) | ControlFlow::Continue(line) => line,
    }
}

pub fn source_offset_to_virtual_column<'a>(offset: usize, line: &VirtualLine<'a>) -> Option<usize> {
    let virtual_col = line.virtual_spans().iter().try_fold(0, |acc, span| {
        match span
            .source_range()
            .filter(|span_range| offset >= span_range.start && offset <= span_range.end)
        {
            Some(source_range) => {
                let byte_offset = offset.saturating_sub(source_range.start);

                let n = span
                    .char_indices()
                    .map_while(|(byte_idx, c)| {
                        (byte_idx < byte_offset).then(|| c.width().unwrap_or(0))
                    })
                    .sum::<usize>();

                ControlFlow::Break(acc + n)
            }
            _ => ControlFlow::Continue(acc + span.width()),
        }
    });

    virtual_col.break_value()
}

fn snap_to_char_boundary(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    (offset..=text.len())
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(0)
}

impl Cursor {
    pub fn new(source_offset: usize) -> Self {
        Self {
            source_offset,
            ..Default::default()
        }
    }

    fn update_virtual_position(&mut self, lines: &[VirtualLine]) {
        if let Some((row, line)) = source_offset_to_virtual_line(self.source_offset, lines) {
            if let Some(col) = source_offset_to_virtual_column(self.source_offset, line) {
                self.virtual_column = col;
            }
            self.virtual_row = row
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        lines: &[VirtualLine],
        text_buffer: &Option<TextBuffer>,
    ) {
        use Message::*;

        match message {
            MoveLeft(amount) => {
                if let Some(text_buffer) = text_buffer {
                    let byte_idx = self
                        .source_offset
                        .saturating_sub(text_buffer.source_range.start);

                    let bytes_amount = text_buffer.content[..byte_idx]
                        .chars()
                        .rev()
                        .take(amount)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();

                    self.source_offset = self.source_offset.saturating_sub(bytes_amount);
                    self.update_virtual_position(lines);
                }
            }

            MoveRight(amount) => {
                if let Some(text_buffer) = text_buffer {
                    let byte_idx = self
                        .source_offset
                        .saturating_sub(text_buffer.source_range.start);

                    let bytes_amount = text_buffer.content[byte_idx..]
                        .chars()
                        .take_while(|&c| c != '\n')
                        .take(amount)
                        .map(|c| c.len_utf8())
                        .sum::<usize>();

                    self.source_offset =
                        (self.source_offset + bytes_amount).min(text_buffer.source_range.end);
                    self.update_virtual_position(lines);
                }
            }

            // TODO: Applies to both cursor_up and cursor_down
            // The cursor should always be fixed to the viewport. This would enable easier implementation
            // for e.g. search feature when navigating between matches
            MoveUp(amount) => {
                let current_idx = self.virtual_row;
                let target_idx = current_idx.saturating_sub(amount);

                for idx in (0..=target_idx).rev() {
                    if let Some(line) = lines.get(idx).filter(|line| line.has_content()) {
                        self.virtual_row = idx;

                        if let Some(source_range) = line.source_range() {
                            match self.mode() {
                                CursorMode::Read => self.source_offset = source_range.start,
                                CursorMode::Edit => {
                                    if let Some(offset) = virtual_position_to_source_offset(
                                        (self.virtual_row, self.virtual_column),
                                        lines,
                                    ) {
                                        self.source_offset = offset;
                                        if let Some(col) = source_offset_to_virtual_column(
                                            self.source_offset,
                                            line,
                                        ) {
                                            self.virtual_column = col;
                                        }
                                    }
                                }
                            }
                        }

                        return;
                    }
                }
            }

            // TODO: Implement scroll offset so that the file scroll offset can be changed by moving
            // cursor downwards when we are at the bottom.
            MoveDown(amount) => {
                let current_idx = self.virtual_row;
                let target_idx = current_idx.saturating_add(amount).min(lines.len());

                for (idx, line) in lines.iter().enumerate().skip(target_idx) {
                    if line.has_content() {
                        self.virtual_row = idx;

                        if let Some(source_range) = line.source_range() {
                            match self.mode() {
                                CursorMode::Read => self.source_offset = source_range.start,
                                CursorMode::Edit => {
                                    if let Some(offset) = virtual_position_to_source_offset(
                                        (self.virtual_row, self.virtual_column),
                                        lines,
                                    ) {
                                        self.source_offset = offset;
                                        if let Some(col) = source_offset_to_virtual_column(
                                            self.source_offset,
                                            line,
                                        ) {
                                            self.virtual_column = col;
                                        }
                                    }
                                }
                            }
                        }

                        return;
                    }
                }
            }

            SwitchMode(CursorMode::Read) => {
                // TODO: Use direction enum to determine, which was the last know direction where the
                // cursor was heading, this way we can select should we check prev or next line if the
                // current_line does not have content.
                if let Some((row, _)) = source_offset_to_virtual_line(self.source_offset, lines) {
                    self.virtual_row = row;
                }

                self.virtual_column = 0;
                self.mode = CursorMode::Read;
            }

            SwitchMode(CursorMode::Edit) => {
                if let Some(text_buffer) = text_buffer {
                    self.source_offset = self
                        .source_offset
                        .clamp(text_buffer.source_range.start, text_buffer.source_range.end);

                    self.update_virtual_position(lines);

                    self.mode = CursorMode::Edit;
                }
            }

            Jump(source_offset) => {
                self.source_offset = source_offset;
                self.update_virtual_position(lines);
            }

            MoveWordForward => {
                if let Some(text_buffer) = text_buffer {
                    let offset = snap_to_char_boundary(
                        &text_buffer.content,
                        self.source_offset
                            .saturating_sub(text_buffer.source_range.start),
                    );

                    let mut chars = text_buffer.content[offset..].char_indices();

                    let byte_idx = chars
                        .by_ref()
                        .find(|&(_, c)| c == ' ')
                        .map(|(i, _)| offset + i + 1);

                    match byte_idx {
                        Some(byte_idx) => {
                            self.source_offset = text_buffer.source_range.start + byte_idx
                        }
                        _ => self.source_offset = text_buffer.source_range.end.saturating_sub(1),
                    }

                    self.update_virtual_position(lines);
                }
            }

            MoveWordBackward => {
                if let Some(text_buffer) = text_buffer {
                    let offset = snap_to_char_boundary(
                        &text_buffer.content,
                        self.source_offset
                            .saturating_sub(text_buffer.source_range.start),
                    );

                    let mut chars = text_buffer.content[..offset].char_indices().rev();

                    let byte_idx = chars
                        .by_ref()
                        .try_fold(false, |found_whitespace, (byte_idx, c)| match c {
                            ' ' if found_whitespace => ControlFlow::Break(offset - byte_idx - 1),
                            ' ' => ControlFlow::Continue(true),
                            _ => ControlFlow::Continue(found_whitespace),
                        })
                        .break_value();

                    match byte_idx {
                        Some(byte_idx) => self.source_offset -= byte_idx,
                        _ => self.source_offset = text_buffer.source_range.start,
                    }

                    self.update_virtual_position(lines);
                }
            }
        };
    }

    pub fn mode(&self) -> &CursorMode {
        &self.mode
    }

    pub fn source_offset(&self) -> usize {
        self.source_offset
    }

    pub fn virtual_row(&self) -> usize {
        self.virtual_row
    }

    pub fn virtual_column(&self) -> usize {
        self.virtual_column
    }
}

#[derive(Clone, Debug, Default)]
pub struct CursorWidget {
    offset: Offset,
}

impl CursorWidget {
    pub fn with_offset(self, offset: Offset) -> Self {
        Self { offset }
    }
}

impl StatefulWidget for CursorWidget {
    type State = Cursor;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let x = area.x.saturating_add(self.offset.x as u16);
        let y = state
            .virtual_row
            .saturating_sub(area.top() as usize)
            .saturating_add(self.offset.y as usize) as u16;

        match state.mode {
            CursorMode::Read => {
                buf.set_style(
                    Rect::new(x, y, area.width, 1),
                    Style::default().reversed().dark_gray(),
                );
            }
            CursorMode::Edit => {
                buf.set_style(
                    Rect::new(x.saturating_add(state.virtual_column as u16), y, 1, 1),
                    Style::default().reversed().dark_gray(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::text::Span;

    use super::*;
    use crate::note_editor::{
        parser,
        render::{render_node, RenderStyle},
        text_buffer::TextBuffer,
    };

    fn render_lines(content: &str) -> Vec<VirtualLine<'static>> {
        parser::from_str(content)
            .into_iter()
            .flat_map(|node| {
                render_node(
                    content.to_string(),
                    &node,
                    80,
                    Span::default(),
                    &RenderStyle::Raw,
                )
                .lines
            })
            .collect()
    }

    /// Renders content with cursor shown as █ at the given byte offset.
    fn render_cursor(content: &str, cursor_offset: usize) -> String {
        use crate::note_editor::virtual_document::VirtualSpan;
        use unicode_width::UnicodeWidthChar;

        let lines = render_lines(content);
        let mut cursor = Cursor::new(cursor_offset);
        cursor.mode = CursorMode::Edit;
        cursor.update_virtual_position(&lines);

        let line_text: String = lines[cursor.virtual_row]
            .virtual_spans()
            .iter()
            .map(|span| match span {
                VirtualSpan::Content(s, _) | VirtualSpan::Synthetic(s) => s.content.to_string(),
            })
            .collect();

        let (result, final_col) = line_text
            .chars()
            .fold((String::new(), 0), |(mut s, col), ch| {
                s.push(if col == cursor.virtual_column {
                    '█'
                } else {
                    ch
                });
                (s, col + ch.width().unwrap_or(0))
            });

        if final_col == cursor.virtual_column {
            result + "█"
        } else {
            result
        }
    }

    #[test]
    fn test_cursor_visual_positions() {
        let test_cases = [
            // (content, cursor_offset, expected)
            ("Hello 😀😀 world", 0, "█ello 😀😀 world"),
            ("Hello 😀😀 world", 6, "Hello █😀 world"),
            ("Hello 😀😀 world", 10, "Hello 😀█ world"),
            ("Hello 😀😀 world", 15, "Hello 😀😀 █orld"),
            ("Hello 😀😀 lorem ipsum\nWhat a day", 27, "█hat a day"),
            ("Hello 😀😀 lorem ipsum\nWhat a day", 32, "What █ day"),
            ("Hello 😀😀 lorem ipsum\nWhat a day", 36, "What a da█"),
            ("Hello world\nSecond line", 0, "█ello world"),
            ("Hello world\nSecond line", 12, "█econd line"),
        ];

        test_cases
            .into_iter()
            .for_each(|(content, cursor_offset, expected)| {
                assert_eq!(render_cursor(content, cursor_offset), expected)
            })
    }

    #[test]
    fn test_virtual_position_to_source_offset() {
        let test_cases = [
            // (content, virtual_position (row, col), expected_byte_offset)
            ("", (0, 0), None),
            ("Hello 😀😀 world", (0, 0), Some(0)),
            ("Hello 😀😀 world", (0, 6), Some(6)),
            ("Hello 😀😀 world", (0, 8), Some(10)),
            ("Hello 😀😀 world", (0, 10), Some(14)),
            ("Hello", (0, 5), Some(5)),
            ("Hello", (0, 6), Some(5)),
            ("Hello\nWhat a day", (1, 0), Some(6)),
            ("Hello😀\nWhat a day", (1, 0), Some(10)),
            ("Hello\nWhat a day", (1, 4), Some(10)),
            ("Hello\nWhat a day", (1, 16), Some(16)),
            ("Hello\nWhat a day", (2, 0), None),
        ];

        test_cases
            .into_iter()
            .for_each(|(content, virtual_position, expected)| {
                assert_eq!(
                    virtual_position_to_source_offset(virtual_position, &render_lines(content)),
                    expected
                )
            });
    }

    #[test]
    fn test_source_offset_to_virtual_column() {
        let test_cases = [
            // (content, byte_offset, expected_virtual_column)
            ("Hello 😀😀 world", 0, Some(0)),
            ("Hello 😀😀 world", 6, Some(6)),
            ("Hello 😀😀 world", 10, Some(8)),
            ("Hello 😀😀 world", 14, Some(10)),
            ("Hello", 5, Some(5)),
            ("Hello", 6, None),
        ];

        test_cases
            .into_iter()
            .for_each(|(content, byte_offset, expected)| {
                assert_eq!(
                    source_offset_to_virtual_column(byte_offset, &render_lines(content)[0]),
                    expected
                )
            });
    }

    #[test]
    fn test_move_right_ascii() {
        let content = "Hello";
        let lines = render_lines(content);
        let text_buffer = TextBuffer::new(content, 0..5);
        let mut cursor = Cursor::new(0);
        cursor.update(Message::MoveRight(1), &lines, &Some(text_buffer));
        assert_eq!(cursor.source_offset, 1);
    }

    #[test]
    fn test_move_right_over_emoji() {
        // "AB😀CD" - emoji at bytes 2-5
        let content = "AB😀CD";
        let lines = render_lines(content);
        let text_buffer = TextBuffer::new(content, 0..10);
        let mut cursor = Cursor::new(2);

        cursor.update(Message::MoveRight(1), &lines, &Some(text_buffer));
        assert_eq!(cursor.source_offset, 6);
    }

    #[test]
    fn test_move_left_ascii() {
        let content = "Hello";
        let lines = render_lines(content);
        let text_buffer = TextBuffer::new(content, 0..5);
        let mut cursor = Cursor::new(3);

        cursor.update(Message::MoveLeft(1), &lines, &Some(text_buffer));
        assert_eq!(cursor.source_offset, 2);
    }

    #[test]
    fn test_move_left_over_emoji() {
        // "AB😀CD" - emoji at bytes 2-5, C at byte 6
        let content = "AB😀CD";
        let lines = render_lines(content);
        let text_buffer = TextBuffer::new(content, 0..10);
        let mut cursor = Cursor::new(6);

        cursor.update(Message::MoveLeft(1), &lines, &Some(text_buffer));
        assert_eq!(cursor.source_offset, 2);
    }

    #[test]
    fn test_move_right_clamped_at_end() {
        let content = "Hello";
        let lines = render_lines(content);
        let text_buffer = TextBuffer::new(content, 0..5);
        let mut cursor = Cursor::new(5);

        cursor.update(Message::MoveRight(1), &lines, &Some(text_buffer));
        assert_eq!(cursor.source_offset, 5);
    }

    #[test]
    fn test_move_left_clamped_at_start() {
        let content = "Hello";
        let lines = render_lines(content);
        let text_buffer = TextBuffer::new(content, 0..5);
        let mut cursor = Cursor::new(0);

        cursor.update(Message::MoveLeft(1), &lines, &Some(text_buffer));
        assert_eq!(cursor.source_offset, 0);
    }

    #[test]
    fn test_move_multiple_characters() {
        let content = "Hello world";
        let lines = render_lines(content);
        let text_buffer = TextBuffer::new(content, 0..11);
        let mut cursor = Cursor::new(0);

        cursor.update(Message::MoveRight(5), &lines, &Some(text_buffer));
        assert_eq!(cursor.source_offset, 5);
    }

    #[test]
    fn test_move_up_empty_line_in_code_block() {
        // Code block: "```rust\nline1\n\nline3\n```"
        // Line 3 (line3) starts at byte 15, empty line at byte 14
        let content = "```rust\nline1\n\nline3\n```";
        let lines = render_lines(content);
        let text_buffer = TextBuffer::new(content, 0..content.len());

        let mut cursor = Cursor::new(15);
        cursor.mode = CursorMode::Edit;
        cursor.update_virtual_position(&lines);
        assert_eq!(cursor.virtual_row, 3);

        // Move up to empty line
        cursor.update(Message::MoveUp(1), &lines, &Some(text_buffer.clone()));
        assert_eq!(cursor.virtual_row, 2);
        // Source offset should be within empty line's range (14..15)
        assert_eq!(cursor.source_offset, 14);

        // Move up to line1
        cursor.update(Message::MoveUp(1), &lines, &Some(text_buffer));
        assert_eq!(cursor.virtual_row, 1);
    }

    #[test]
    fn test_move_up_from_empty_line() {
        let content = "```rust\nline1\n\nline3\n```";
        let lines = render_lines(content);
        let text_buffer = TextBuffer::new(content, 0..content.len());

        // Start on empty line (byte 14)
        let mut cursor = Cursor::new(14);
        cursor.mode = CursorMode::Edit;
        cursor.update_virtual_position(&lines);
        assert_eq!(cursor.virtual_row, 2);

        cursor.update(Message::MoveUp(1), &lines, &Some(text_buffer));
        assert_eq!(cursor.virtual_row, 1);
    }

    fn render_lines_visual(content: &str) -> Vec<VirtualLine<'static>> {
        parser::from_str(content)
            .into_iter()
            .flat_map(|node| {
                render_node(
                    content.to_string(),
                    &node,
                    80,
                    Span::default(),
                    &RenderStyle::Visual,
                )
                .lines
            })
            .collect()
    }

    #[test]
    fn test_move_up_empty_line_visual_mode() {
        // Visual mode has padding lines, so structure differs from raw
        let content = "```rust\nline1\n\nline3\n```";
        let lines = render_lines_visual(content);

        // In visual mode: line 0 = padding, line 1 = line1, line 2 = empty, line 3 = line3
        // Source ranges: line1 = 0..6, empty = 6..7, line3 = 7..13
        let mut cursor = Cursor::new(7); // Start of line3 in visual source ranges
        cursor.mode = CursorMode::Read;
        cursor.update_virtual_position(&lines);
        assert_eq!(cursor.virtual_row, 3);

        cursor.update(Message::MoveUp(1), &lines, &None);
        assert_eq!(cursor.virtual_row, 2);

        cursor.update(Message::MoveUp(1), &lines, &None);
        assert_eq!(cursor.virtual_row, 1);
    }
}
