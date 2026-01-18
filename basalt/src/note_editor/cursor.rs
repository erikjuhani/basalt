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

    for span in line.virtual_spans() {
        match span.source_range() {
            Some(range) => {
                for (byte_idx, ch) in span.char_indices() {
                    if cur_col >= col {
                        return Some(range.start + byte_idx);
                    }

                    let char_width = ch.width().unwrap_or(0);
                    cur_col += char_width;
                }
            }
            _ => cur_col += span.width(),
        }
    }

    // If we've processed all spans and still haven't reached the target column,
    // we're at the end of the line. For empty lines or when col is 0, use start.
    // Otherwise use end-1 to avoid placing cursor on the newline character.
    if col == 0 || cur_col == 0 {
        return Some(source_range.start);
    }
    Some(source_range.end.saturating_sub(1))
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
                let idx = offset.saturating_sub(source_range.start);
                let n = span
                    .chars()
                    .take(idx)
                    .map(|c| c.width().unwrap_or(0))
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
                    self.source_offset = self
                        .source_offset
                        .saturating_sub(amount)
                        .max(text_buffer.source_range.start);

                    self.update_virtual_position(lines);
                }
            }

            MoveRight(amount) => {
                if let Some(text_buffer) = text_buffer {
                    self.source_offset = self
                        .source_offset
                        .saturating_add(amount)
                        .min(text_buffer.source_range.end.saturating_sub(1));

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
