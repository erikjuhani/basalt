use std::time::{Duration, Instant};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph, Widget},
};

use crate::{app::Message as AppMessage, config::Symbols};

pub const TOAST_WIDTH: u16 = 40;

#[derive(Clone, PartialEq, Debug)]
pub struct Toast {
    level: Option<ToastLevel>,
    pub(super) message: String,
    pub icon: String,
    created_at: Instant,
    duration: Duration,
    width: usize,
    pub border_type: BorderType,
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

    pub fn level_icon(&self, symbols: &Symbols) -> String {
        match &self.level {
            Some(ToastLevel::Success) => symbols.toast_success.clone(),
            Some(ToastLevel::Info) => symbols.toast_info.clone(),
            Some(ToastLevel::Error) => symbols.toast_error.clone(),
            Some(ToastLevel::Warning) => symbols.toast_warning.clone(),
            None => String::default(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    pub fn height(&self) -> u16 {
        let content_width = TOAST_WIDTH.saturating_sub(6) as usize;
        let wrapped = textwrap::wrap(&self.message, content_width);
        wrapped.len().max(1) as u16 + 2
    }
}

impl Widget for Toast {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let height = self.height();
        let color = self.level.as_ref().map(|l| l.color()).unwrap_or_default();

        let block = Block::bordered()
            .border_type(self.border_type)
            .border_style(Style::new().fg(color));

        let toast_area = Rect {
            x: area.x,
            y: area.y,
            width: TOAST_WIDTH.min(area.width),
            height: height.min(area.height),
        };

        Clear.render(toast_area, buf);

        let content_width = TOAST_WIDTH.saturating_sub(6) as usize;
        let wrapped = textwrap::wrap(&self.message, content_width);

        let lines: Vec<Line> = wrapped
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    Line::from(vec![
                        Span::from(" "),
                        Span::from(self.icon.clone()).fg(color),
                        Span::from(" "),
                        Span::from(line.to_string()),
                    ])
                } else {
                    Line::from(format!("   {line}"))
                }
            })
            .collect();

        Paragraph::new(lines).block(block).render(toast_area, buf);
    }
}

impl Default for Toast {
    fn default() -> Self {
        Self {
            level: Option::default(),
            message: String::default(),
            icon: String::default(),
            created_at: Instant::now(),
            duration: Duration::default(),
            border_type: BorderType::default(),
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

        tests.into_iter().for_each(|(name, mut toast)| {
            _ = terminal.clear();
            terminal
                .draw(|frame| {
                    toast.icon = toast.level_icon(&Symbols::unicode());
                    toast.render(frame.area(), frame.buffer_mut());
                })
                .unwrap();
            assert_snapshot!(name, terminal.backend());
        });
    }
}
