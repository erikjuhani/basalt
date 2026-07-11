use std::ops::Deref;

use ratatui::layout::{Offset, Rect, Size};

#[derive(Clone, Debug, Default)]
pub struct Viewport {
    area: Rect,
}

impl Deref for Viewport {
    type Target = Rect;

    fn deref(&self) -> &Self::Target {
        &self.area
    }
}

impl Viewport {
    pub fn area(&self) -> Rect {
        self.area
    }

    pub fn size_changed(&self, size: Size) -> bool {
        (self.height, self.width) != (size.height, size.width)
    }

    pub fn resize(&mut self, size: Size) {
        self.area.width = size.width;
        self.area.height = size.height;
    }

    pub fn scroll_by(&mut self, offset: (i32, i32)) {
        self.area = self.offset(Offset {
            y: offset.0,
            x: offset.1,
        })
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let scroll = amount.min(self.area.y as usize) as i32;
        if scroll > 0 {
            self.scroll_by((-scroll, 0));
        }
    }

    /// Scrolls down by `amount`, stopping so the top never passes `max_top`.
    pub fn scroll_down(&mut self, amount: usize, max_top: usize) {
        let scroll = amount.min(max_top.saturating_sub(self.area.y as usize)) as i32;
        if scroll > 0 {
            self.scroll_by((scroll, 0));
        }
    }
}
