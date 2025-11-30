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
}
