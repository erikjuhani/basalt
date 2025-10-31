use ratatui::{
    buffer::Buffer,
    layout::{Offset, Rect},
    style::{Style, Stylize},
    widgets::StatefulWidget,
};

use crate::note_editor::virtual_document::{VirtualDocument, VirtualLine};

#[derive(Clone, Debug, Default)]
pub enum CursorMode {
    #[default]
    Read,
    Edit,
}

#[derive(Clone, Debug, Default)]
pub struct Cursor {
    mode: CursorMode,
    source_offset: usize,
    virtual_line: usize,
    virtual_column: usize,
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

    pub fn enter_read_mode(&mut self, virtual_document: &VirtualDocument) {
        for (line_idx, line) in virtual_document.lines().iter().enumerate() {
            if let Some(source_range) = line.source_range() {
                if source_range.contains(&self.source_offset) {
                    self.mode = CursorMode::Read;
                    self.virtual_line = line_idx;
                    return;
                }
            }
        }

        self.mode = CursorMode::Read;
    }

    pub fn enter_edit_mode(&mut self, virtual_document: &VirtualDocument) {
        let line_pos = self.virtual_line;
        if let Some((_, block)) = virtual_document.get_block(line_pos) {
            let source_range = block.source_range();
            self.source_offset = self
                .source_offset
                .clamp(source_range.start, source_range.end);

            self.mode = CursorMode::Edit;
        }
    }

    pub fn find_source_line<'a>(
        &self,
        lines: &[VirtualLine<'a>],
    ) -> Option<(usize, VirtualLine<'a>)> {
        for (idx, line) in lines.iter().enumerate() {
            if let Some(source_range) = line.source_range() {
                if source_range.contains(&self.source_offset) {
                    return Some((idx, line.clone()));
                }
            }
        }

        None
    }

    pub fn find_source_column(&self, line: VirtualLine) -> Option<usize> {
        Some(line.virtual_spans().iter().fold(0, |acc, span| {
            if let Some(source_range) = span.source_range() {
                if source_range.contains(&self.source_offset) {
                    // println!("{:?}", acc + self.source_offset);
                    acc + self.source_offset.saturating_sub(source_range.start)
                } else {
                    acc + span.width()
                }
            } else {
                acc + span.width()
            }
        }))
    }

    // pub fn cursor_left(&mut self, amount: usize, lines: &[VirtualLine]) {
    //     todo!()
    // }
    //
    // pub fn cursor_right(&mut self, amount: usize, lines: &[VirtualLine]) {
    //     todo!()
    // }

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
            CursorMode::Edit => {}
        }
    }

    // TODO: Implement scroll offset so that the file scroll offset can be changed by moving
    // cursor downwards when we are at the bottom.
    pub fn cursor_down(&mut self, amount: usize, lines: &[VirtualLine]) {
        match self.mode {
            CursorMode::Read => {
                let current_idx = self.virtual_line;
                let target_idx = current_idx
                    .saturating_add(amount)
                    .min(lines.len().saturating_sub(2));

                for (idx, line) in lines.iter().enumerate().skip(target_idx) {
                    if line.has_content() {
                        self.virtual_line = idx;

                        if let Some(source_range) = lines[idx].source_range() {
                            self.source_offset = source_range.start;
                        }

                        return;
                    }
                }
            }
            CursorMode::Edit => {}
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
