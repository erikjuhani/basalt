use std::borrow::Cow;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
};

use crate::config::Theme;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scroll {
    pub content_length: usize,
    pub offset: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Panel<'a> {
    title: Option<Cow<'a, str>>,
    label: Option<Cow<'a, str>>,
    focused: bool,
    theme: Theme,
    scroll: Option<Scroll>,
}

impl<'a> Panel<'a> {
    pub fn new(theme: Theme) -> Self {
        Self {
            title: None,
            label: None,
            focused: false,
            theme,
            scroll: None,
        }
    }

    pub fn title(self, title: impl Into<Cow<'a, str>>) -> Self {
        Self {
            title: Some(title.into()),
            ..self
        }
    }

    pub fn label(self, label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label: Some(label.into()),
            ..self
        }
    }

    pub fn focused(self, focused: bool) -> Self {
        Self { focused, ..self }
    }

    pub fn scroll(self, scroll: Scroll) -> Self {
        Self {
            scroll: Some(scroll),
            ..self
        }
    }

    /// The content area leaves out the scrollbar column when the content overflows.
    pub fn inner(&self, area: Rect) -> Rect {
        let inner = Block::bordered().inner(area);
        if self.overflows(inner) {
            Rect {
                width: inner.width.saturating_sub(1),
                ..inner
            }
        } else {
            inner
        }
    }

    fn overflows(&self, inner: Rect) -> bool {
        self.scroll
            .is_some_and(|scroll| scroll.content_length > inner.height.into())
    }

    fn highlight(&self, unfocused: Color) -> Color {
        if self.focused {
            self.theme.accent
        } else {
            unfocused
        }
    }

    fn title_style(&self) -> Style {
        let style = Style::new().fg(self.highlight(self.theme.muted));
        if self.focused {
            style.bold()
        } else {
            style
        }
    }

    fn block(&self) -> Block<'a> {
        let border = self.highlight(self.theme.border);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(border);

        let block = match &self.title {
            Some(title) => block.title(Line::from(vec![
                Span::styled("─", border),
                Span::styled(format!(" {title} "), self.title_style()),
            ])),
            None => block,
        };

        match &self.label {
            Some(label) => block.title(
                Line::styled(format!(" {label} "), self.theme.muted).alignment(Alignment::Right),
            ),
            None => block,
        }
    }
}

impl Widget for Panel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = Block::bordered().inner(area);
        self.block().render(area, buf);

        if let Some(scroll) = self.scroll.filter(|_| self.overflows(inner)) {
            let viewport = usize::from(inner.height);
            let mut state = ScrollbarState::new(scroll.content_length - viewport + 1)
                .position(scroll.offset)
                .viewport_content_length(viewport);

            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .track_style(self.theme.border)
                .thumb_symbol("┃")
                .thumb_style(self.highlight(self.theme.muted))
                .render(inner, buf, &mut state);
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;

    fn render(panel: Panel) -> Buffer {
        let area = Rect::new(0, 0, 24, 6);
        let mut buffer = Buffer::empty(area);
        panel.render(area, &mut buffer);
        buffer
    }

    fn panel(focused: bool) -> Panel<'static> {
        let theme = Theme {
            accent: Color::Magenta,
            border: Color::DarkGray,
            muted: Color::Gray,
            ..Theme::default()
        };

        Panel::new(theme)
            .title("Explorer")
            .label("docs")
            .focused(focused)
    }

    #[test]
    fn test_render_focused() {
        assert_snapshot!(format!("{:?}", render(panel(true))));
    }

    #[test]
    fn test_render_unfocused() {
        assert_snapshot!(format!("{:?}", render(panel(false))));
    }

    #[test]
    fn test_render_focused_overflow() {
        let scroll = Scroll {
            content_length: 16,
            offset: 6,
        };
        assert_snapshot!(format!("{:?}", render(panel(true).scroll(scroll))));
    }

    #[test]
    fn test_render_unfocused_overflow() {
        let scroll = Scroll {
            content_length: 16,
            offset: 0,
        };
        assert_snapshot!(format!("{:?}", render(panel(false).scroll(scroll))));
    }

    #[test]
    fn test_render_content_fits() {
        let scroll = Scroll {
            content_length: 4,
            offset: 0,
        };
        assert_eq!(render(panel(true).scroll(scroll)), render(panel(true)));
    }

    #[test]
    fn test_inner() {
        let area = Rect::new(0, 0, 24, 6);
        let overflow = Scroll {
            content_length: 5,
            offset: 0,
        };
        let fits = Scroll {
            content_length: 4,
            offset: 0,
        };

        assert_eq!(panel(true).inner(area), Rect::new(1, 1, 22, 4));
        assert_eq!(panel(true).scroll(fits).inner(area), Rect::new(1, 1, 22, 4));
        assert_eq!(
            panel(true).scroll(overflow).inner(area),
            Rect::new(1, 1, 21, 4)
        );
    }
}
