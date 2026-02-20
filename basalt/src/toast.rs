use std::time::{Duration, Instant};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph, Widget},
};

use crate::app::Message as AppMessage;

pub const TOAST_WIDTH: u16 = 40;
pub const TOAST_HEIGHT: u16 = 3;

#[derive(Clone, PartialEq, Debug)]
pub struct Toast {
    level: Option<ToastLevel>,
    pub(super) message: String,
    created_at: Instant,
    duration: Duration,
    width: usize,
}

impl Toast {
    pub fn new(message: &str, duration: Duration) -> Self {
        Self {
            message: message.to_string(),
            duration,
            ..Default::default()
        }
    }

    pub fn info(message: &str, duration: Duration) -> Self {
        Self {
            level: Some(ToastLevel::Info),
            ..Toast::new(message, duration)
        }
    }

    pub fn warn(message: &str, duration: Duration) -> Self {
        Self {
            level: Some(ToastLevel::Warning),
            ..Toast::new(message, duration)
        }
    }

    pub fn error(message: &str, duration: Duration) -> Self {
        Self {
            level: Some(ToastLevel::Error),
            ..Toast::new(message, duration)
        }
    }

    pub fn success(message: &str, duration: Duration) -> Self {
        Self {
            level: Some(ToastLevel::Success),
            ..Toast::new(message, duration)
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }
}

impl Widget for Toast {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let (icon, color) = if let Some(level) = self.level {
            (level.icon(), level.color())
        } else {
            ("", Color::default())
        };

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(color));

        let toast_area = Rect {
            x: area.x,
            y: area.y,
            width: TOAST_WIDTH.min(area.width),
            height: TOAST_HEIGHT.min(area.height),
        };

        Clear.render(toast_area, buf);

        let content = Line::from(vec![
            Span::from(" "),
            Span::from(icon).fg(color),
            Span::from(" "),
            Span::from(self.message),
            Span::from(" "),
        ]);

        Paragraph::new(content).block(block).render(toast_area, buf);
    }
}

impl Default for Toast {
    fn default() -> Self {
        Self {
            level: Option::default(),
            message: String::default(),
            created_at: Instant::now(),
            duration: Duration::default(),
            width: 30,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum ToastLevel {
    Success,
    Info,
    Warning,
    Error,
}

impl ToastLevel {
    pub fn icon(&self) -> &'static str {
        match self {
            ToastLevel::Success => "✓",
            ToastLevel::Info => "ⓘ",
            ToastLevel::Error => "✗",
            ToastLevel::Warning => "⚠",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ToastLevel::Success => Color::Green,
            ToastLevel::Info => Color::Blue,
            ToastLevel::Error => Color::Red,
            ToastLevel::Warning => Color::Yellow,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Message {
    Create(Toast),
    Tick,
}

pub fn update<'a>(message: Message, state: &mut Vec<Toast>) -> Option<AppMessage<'a>> {
    match message {
        Message::Create(toast) => {
            state.push(toast);
        }
        Message::Tick => {
            state.retain(|toast| !toast.is_expired());
        }
    };
    None
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    use crate::toast::{update, Message, Toast};

    #[test]
    fn test_toast_update_expired() {
        let mut state = vec![];
        update(Message::Create(Toast::default()), &mut state);
        assert_eq!(state.len(), 1);
        sleep(Duration::from_millis(1));
        update(Message::Tick, &mut state);
        assert_eq!(state.len(), 0);
    }

    #[test]
    fn test_toast_update_not_expired() {
        let mut state = vec![];
        update(
            Message::Create(Toast::new("Toast B", Duration::from_secs(10))),
            &mut state,
        );
        update(Message::Tick, &mut state);
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn test_toast_render() {
        let width = 50;
        let height = 3;
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();

        let tests: Vec<(&str, Toast)> = vec![
            ("info", Toast::info("File saved", Duration::from_secs(5))),
            (
                "error",
                Toast::error("Failed to save file", Duration::from_secs(5)),
            ),
            (
                "warning",
                Toast::warn("Unsaved changes", Duration::from_secs(5)),
            ),
            (
                "success",
                Toast::success("Operation complete", Duration::from_secs(5)),
            ),
            (
                "long_message",
                Toast::info(
                    "This is a really long message that should be truncated",
                    Duration::from_secs(5),
                ),
            ),
            (
                "no_level",
                Toast::new("Plain toast", Duration::from_secs(5)),
            ),
        ];

        tests.into_iter().for_each(|(name, toast)| {
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    toast.render(frame.area(), frame.buffer_mut());
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }
}
