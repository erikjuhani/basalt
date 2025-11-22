use std::ops::ControlFlow;

use ratatui::{
    buffer::Buffer,
    layout::{Offset, Rect},
    style::{Style, Stylize},
    widgets::StatefulWidget,
};
use unicode_width::UnicodeWidthChar;

use crate::note_editor::{
    editor::TextBuffer,
    virtual_document::{VirtualDocument, VirtualLine},
};

#[derive(Clone, Debug)]
pub enum CursorMove {
    Top,
    Bottom,
    WordForward,
    WordBackward,
    Up(usize),
    Down(usize),
    Left(usize),
    Right(usize),
    Move(i32, i32),
    Jump(u16, u16),
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
    pub virtual_line: usize,
    pub virtual_column: usize,
}

pub fn offset_to_virtual_line<'a>(offset: usize, lines: &[VirtualLine<'a>]) -> Option<usize> {
    lines.iter().enumerate().find_map(|(row, line)| {
        let source_range = line.source_range()?;
        // FIXME: A minor bug happens if user writes whitespace on a line and a word is then wrapped to
        // next line. The cursor is also wrapped to next line.
        (offset >= source_range.start && offset <= source_range.end).then_some(row)
    })
}

pub fn offset_to_virtual_column<'a>(offset: usize, line: &VirtualLine<'a>) -> Option<usize> {
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
                    .map(|c| c.width().unwrap_or_default())
                    .sum::<usize>();

                ControlFlow::Break(acc + n)
            }
            _ => ControlFlow::Continue(acc + span.width()),
        }
    });

    match virtual_col {
        ControlFlow::Break(col) => Some(col),
        _ => None,
    }
}

pub fn offset_to_virtual_position<'a>(
    offset: usize,
    lines: &[VirtualLine<'a>],
) -> Option<(usize, usize)> {
    lines.iter().enumerate().find_map(|(row, line)| {
        let line_range = line.source_range()?;

        if offset >= line_range.start && offset <= line_range.end {
            let virtual_col = line.virtual_spans().iter().try_fold(0, |acc, span| {
                match span
                    .source_range()
                    .filter(|span_range| offset >= span_range.start && offset < span_range.end)
                {
                    Some(source_range) => {
                        let idx = offset.saturating_sub(source_range.start);
                        let n = span
                            .chars()
                            .take(idx)
                            .map(|c| c.width().unwrap_or_default())
                            .sum::<usize>();

                        ControlFlow::Break(acc + n)
                    }
                    _ => ControlFlow::Continue(acc + span.width()),
                }
            });

            match virtual_col {
                ControlFlow::Break(col) => Some((row, col)),
                _ => None,
            }
        } else {
            None
        }
    })
}

impl Cursor {
    pub fn new(source_offset: usize) -> Self {
        Self {
            source_offset,
            ..Default::default()
        }
    }

    pub fn mode(&self) -> &CursorMode {
        &self.mode
    }

    pub fn source_offset(&self) -> usize {
        self.source_offset
    }

    pub fn virtual_line(&self) -> usize {
        self.virtual_line
    }

    pub fn virtual_column(&self) -> usize {
        self.virtual_column
    }

    pub fn move_action(
        &mut self,
        move_action: CursorMove,
        lines: &[VirtualLine],
        buffer: &Option<TextBuffer>,
    ) {
        use CursorMove::*;

        match move_action {
            Top => {
                self.source_offset = 0;
                self.virtual_line = 0;
                self.virtual_column = 0;
            }
            Bottom => {
                // ^ Maybe do something similar to top
                self.cursor_down(lines.len(), lines);
            }
            Up(amount) => {
                self.cursor_up(amount, lines);
            }
            Down(amount) => {
                self.cursor_down(amount, lines);
            }
            Left(amount) => {
                if let Some(text_buffer) = buffer {
                    self.cursor_left(amount, lines, text_buffer);
                }
            }
            Right(amount) => {
                if let Some(text_buffer) = buffer {
                    self.cursor_right(amount, lines, text_buffer);
                }
            }
            WordForward | WordBackward => {}
            _ => {}
        }
    }

    pub fn enter_read_mode(&mut self, virtual_document: &VirtualDocument) {
        if let Some(row) = offset_to_virtual_line(self.source_offset(), virtual_document.lines()) {
            self.virtual_line = row;
        } else if let Some((next_line, source_offset)) =
            self.prev_available_line(0, virtual_document.lines())
        {
            self.virtual_line = next_line;
            self.source_offset = source_offset;
        } else if let Some((prev_line, source_offset)) =
            self.next_available_line(0, virtual_document.lines())
        {
            self.virtual_line = prev_line;
            self.source_offset = source_offset;
        }

        self.mode = CursorMode::Read;
    }

    pub fn enter_edit_mode(&mut self, virtual_document: &VirtualDocument) {
        let Some(block_idx) = virtual_document.line_to_block().get(self.virtual_line) else {
            return;
        };
        if let Some((_, block)) = virtual_document.get_block(*block_idx) {
            let source_range = block.source_range();
            self.source_offset = self
                .source_offset
                .clamp(source_range.start, source_range.end);

            if let Some((_, col)) = offset_to_virtual_position(self.source_offset(), block.lines())
            {
                self.virtual_column = col;
            }

            self.mode = CursorMode::Edit;
        }
    }

    pub fn find_source_line<'a>(
        &self,
        lines: &[VirtualLine<'a>],
    ) -> Option<(usize, VirtualLine<'a>)> {
        for (idx, line) in lines.iter().enumerate() {
            if let Some(source_range) = line.source_range() {
                if source_range.start <= self.source_offset
                    && self.source_offset <= source_range.end
                {
                    return Some((idx, line.clone()));
                }
            }
        }

        None
    }

    pub fn find_source_column(&self, line: VirtualLine) -> Option<usize> {
        let mut width = 0;

        for span in line.virtual_spans() {
            if let Some(source_range) = span.source_range() {
                if source_range.start <= self.source_offset
                    && self.source_offset <= source_range.end
                {
                    return Some(width + self.source_offset.saturating_sub(source_range.start));
                }
            }
            width += span.width();
        }

        None
    }

    // FIXME: Currently we consider new line characters as part of the source buffer when
    // calculating the virtual cursor location, however, this appears as an ui/ux bug when
    // travelling to next line using left or right movement. The cursor appears to be 'stuck',
    // since we don't render new line characters. For movement purposes the new line characters
    // should be skipped in the source_range.
    pub fn cursor_left(&mut self, amount: usize, lines: &[VirtualLine], buffer: &TextBuffer) {
        self.source_offset = self
            .source_offset
            .saturating_sub(amount)
            .max(buffer.source_range.start);

        if let Some(row) = offset_to_virtual_line(self.source_offset, lines) {
            if let Some(line) = lines.get(row) {
                if let Some(col) = offset_to_virtual_column(self.source_offset, line) {
                    self.virtual_column = col;
                }
            }
            self.virtual_line = row;
        }
    }

    // FIXME: Currently we consider new line characters as part of the source buffer when
    // calculating the virtual cursor location, however, this appears as an ui/ux bug when
    // travelling to next line using left or right movement. The cursor appears to be 'stuck',
    // since we don't render new line characters. For movement purposes the new line characters
    // should be skipped in the source_range.
    pub fn cursor_right(&mut self, amount: usize, lines: &[VirtualLine], buffer: &TextBuffer) {
        self.source_offset = self
            .source_offset
            .saturating_add(amount)
            .min(buffer.source_range.end.saturating_sub(1));

        if let Some(row) = offset_to_virtual_line(self.source_offset, lines) {
            if let Some(line) = lines.get(row) {
                if let Some(col) = offset_to_virtual_column(self.source_offset, line) {
                    self.virtual_column = col;
                    self.virtual_line = row
                }
            }
        }
    }

    /// (virtual_line, source_offset_start)
    pub fn next_available_line(
        &self,
        amount: usize,
        lines: &[VirtualLine],
    ) -> Option<(usize, usize)> {
        let current_idx = self.virtual_line;
        let target_idx = current_idx
            .saturating_add(amount)
            .min(lines.len().saturating_sub(1));

        lines
            .iter()
            .enumerate()
            .skip(target_idx)
            .find_map(|(idx, line)| {
                if line.has_content() {
                    line.source_range()
                        .map(|source_range| (idx, source_range.start))
                } else {
                    None
                }
            })
    }

    pub fn prev_available_line(
        &self,
        amount: usize,
        lines: &[VirtualLine],
    ) -> Option<(usize, usize)> {
        let current_idx = self.virtual_line;
        let target_idx = current_idx.saturating_sub(amount);

        lines
            .iter()
            .enumerate()
            .take(target_idx.saturating_add(1))
            .rev()
            .find_map(|(idx, line)| {
                if line.has_content() {
                    line.source_range()
                        .map(|source_range| (idx, source_range.start))
                } else {
                    None
                }
            })
    }

    // TODO: Applies to both cursor_up and cursor_down
    // The cursor should always be fixed to the viewport. This would enable easier implementation
    // for e.g. search feature when navigating between matches
    pub fn cursor_up(&mut self, amount: usize, lines: &[VirtualLine]) {
        match self.mode {
            CursorMode::Read => {
                let current_idx = self.virtual_line;
                let target_idx = current_idx.saturating_sub(amount);

                for idx in (0..=target_idx).rev() {
                    if lines.get(idx).is_some_and(|line| line.has_content()) {
                        self.virtual_line = idx;

                        if let Some(source_range) = lines[idx].source_range() {
                            self.source_offset = source_range.start;
                        }

                        return;
                    }
                }
            }
            CursorMode::Edit => {
                let current_idx = self.virtual_line;
                let target_idx = current_idx.saturating_sub(amount);

                for idx in (0..=target_idx).rev() {
                    if lines.get(idx).is_some_and(|line| line.has_content()) {
                        self.virtual_line = idx;

                        let prev_line = &lines[current_idx];
                        let line = &lines[idx];

                        if let Some(source_range) = line.source_range() {
                            let column_offset =
                                if let Some(prev_source_range) = prev_line.source_range() {
                                    self.source_offset
                                        .saturating_sub(prev_source_range.start)
                                        .min(line.content_width())
                                } else {
                                    0
                                };
                            self.source_offset = source_range
                                .start
                                .saturating_add(column_offset)
                                .min(source_range.end);

                            if let Some(column) = self.find_source_column(line.clone()) {
                                self.virtual_column = column;
                            }
                        }

                        return;
                    }
                }
            }
        }
    }

    // TODO: Implement scroll offset so that the file scroll offset can be changed by moving
    // cursor downwards when we are at the bottom.
    pub fn cursor_down(&mut self, amount: usize, lines: &[VirtualLine]) {
        // let lines = virtual_document.lines();
        match self.mode {
            CursorMode::Read => {
                let current_idx = self.virtual_line;
                let target_idx = current_idx.saturating_add(amount).min(lines.len());

                for (idx, line) in lines.iter().enumerate().skip(target_idx) {
                    if line.has_content() {
                        self.virtual_line = idx;

                        if let Some(source_range) = line.source_range() {
                            self.source_offset = source_range.start;
                        }

                        return;
                    }
                }
            }
            CursorMode::Edit => {
                let current_idx = self.virtual_line;
                let target_idx = current_idx.saturating_add(amount).min(lines.len());

                for (idx, line) in lines.iter().enumerate().skip(target_idx) {
                    if line.has_content() {
                        let prev_line = &lines[current_idx];
                        self.virtual_line = idx;

                        if let Some(source_range) = line.source_range() {
                            let column_offset =
                                if let Some(prev_source_range) = prev_line.source_range() {
                                    self.source_offset
                                        .saturating_sub(prev_source_range.start)
                                        .min(line.content_width())
                                } else {
                                    0
                                };

                            self.source_offset = source_range.start.saturating_add(column_offset);

                            if let Some(column) = self.find_source_column(line.clone()) {
                                self.virtual_column = column;
                            }

                            return;
                        }
                    }
                }
            }
        }
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
            .virtual_line
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
