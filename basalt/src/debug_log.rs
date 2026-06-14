//! In-TUI debug log overlay.
//!
//! A [`tracing`] [`Layer`] captures every event into a bounded, process-global ring
//! buffer. The [`DebugLogModal`] overlay renders a snapshot of that buffer on top of the
//! application: a live console that can be toggled at any time without interfering with
//! normal usage.

use std::{
    collections::VecDeque,
    fmt::{self, Write},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect, Size},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Widget,
    },
};
use tracing::{field::Field, field::Visit, level_filters::LevelFilter, Event, Subscriber};
use tracing_subscriber::{layer::Context, Layer};

use crate::app::{calc_scroll_amount, Message as AppMessage, ScrollAmount};

/// Maximum number of retained log entries. Oldest entries are evicted past this.
const CAPACITY: usize = 2000;

/// A single captured log record, cheap to clone for rendering snapshots.
#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub elapsed: Duration,
}

/// Severity of a log record. Ordered from least to most severe so that a minimum-level
/// filter is a simple `>=` comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, clap::ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Fixed-width label so columns stay aligned across rows.
    pub fn label(self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO ",
            LogLevel::Warn => "WARN ",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn color(self) -> Color {
        match self {
            LogLevel::Trace => Color::DarkGray,
            LogLevel::Debug => Color::Blue,
            LogLevel::Info => Color::Green,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Error => Color::Red,
        }
    }

    /// Next level in a wrapping cycle, used by the overlay's level filter.
    fn next(self) -> Self {
        match self {
            LogLevel::Trace => LogLevel::Debug,
            LogLevel::Debug => LogLevel::Info,
            LogLevel::Info => LogLevel::Warn,
            LogLevel::Warn => LogLevel::Error,
            LogLevel::Error => LogLevel::Trace,
        }
    }
}

impl From<tracing::Level> for LogLevel {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        }
    }
}

fn buffer() -> &'static Mutex<VecDeque<LogEntry>> {
    static LOG_BUFFER: OnceLock<Mutex<VecDeque<LogEntry>>> = OnceLock::new();
    LOG_BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAPACITY)))
}

/// Process start, used to render a monotonic relative timestamp per entry.
fn start() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// Pushes an entry into a bounded buffer, evicting the oldest once at capacity.
fn push_bounded(buffer: &mut VecDeque<LogEntry>, entry: LogEntry) {
    if buffer.len() == CAPACITY {
        buffer.pop_front();
    }
    buffer.push_back(entry);
}

/// Snapshots the entries matching `min_level`, newest last.
fn snapshot(min_level: LogLevel) -> Vec<LogEntry> {
    buffer()
        .lock()
        .map(|buffer| {
            buffer
                .iter()
                .filter(|entry| entry.level >= min_level)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Empties the ring buffer.
pub fn clear() {
    if let Ok(mut buffer) = buffer().lock() {
        buffer.clear();
    }
}

/// Registers the capturing [`Layer`] as the global tracing subscriber. Call once at startup.
pub fn init() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    start();
    let _ = tracing_subscriber::registry()
        .with(DebugLogLayer)
        .try_init();
}

/// A [`tracing`] layer that records every event into the ring buffer.
struct DebugLogLayer;

impl<S: Subscriber> Layer<S> for DebugLogLayer {
    // Capture everything; the overlay does its own level filtering.
    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::TRACE)
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let metadata = event.metadata();
        let entry = LogEntry {
            level: (*metadata.level()).into(),
            target: metadata.target().to_string(),
            message: format!("{}{}", visitor.message, visitor.fields),
            elapsed: start().elapsed(),
        };

        if let Ok(mut buffer) = buffer().lock() {
            push_bounded(&mut buffer, entry);
        }
    }
}

/// Collects an event's `message` field and appends any structured fields as `key=value`.
#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            let _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Toggle,
    Close,
    Clear,
    CycleLevel,
    ScrollUp(ScrollAmount),
    ScrollDown(ScrollAmount),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugLogModalState {
    pub visible: bool,
    /// Lines scrolled up from the tail; `0` follows the newest entries.
    pub offset: usize,
    pub min_level: LogLevel,
    pub scrollbar_state: ScrollbarState,
}

impl Default for DebugLogModalState {
    fn default() -> Self {
        Self {
            visible: false,
            offset: 0,
            min_level: LogLevel::Trace,
            scrollbar_state: ScrollbarState::default(),
        }
    }
}

impl DebugLogModalState {
    fn scroll_up(&mut self, amount: usize, max_offset: usize) {
        self.offset = (self.offset + amount).min(max_offset);
    }

    fn scroll_down(&mut self, amount: usize) {
        self.offset = self.offset.saturating_sub(amount);
    }

    fn cycle_level(&mut self) {
        self.min_level = self.min_level.next();
        self.offset = 0;
    }
}

pub fn update<'a>(
    message: &Message,
    screen_size: Size,
    state: &mut DebugLogModalState,
) -> Option<AppMessage<'a>> {
    let page = inner_height(overlay_area(Rect::new(
        0,
        0,
        screen_size.width,
        screen_size.height,
    )));
    let total = snapshot(state.min_level).len();
    let max_offset = total.saturating_sub(page);

    match message {
        Message::Toggle => state.visible = !state.visible,
        Message::Close => state.visible = false,
        Message::Clear => {
            clear();
            state.offset = 0;
        }
        Message::CycleLevel => state.cycle_level(),
        Message::ScrollUp(amount) => state.scroll_up(calc_scroll_amount(amount, page), max_offset),
        Message::ScrollDown(amount) => state.scroll_down(calc_scroll_amount(amount, page)),
    };

    None
}

/// Bottom-docked overlay area: lower half of the screen, full width minus the app margin.
fn overlay_area(area: Rect) -> Rect {
    let [area] = Layout::vertical([Constraint::Percentage(50)])
        .flex(Flex::End)
        .areas(area);
    let [area] = Layout::horizontal([Constraint::Fill(1)])
        .horizontal_margin(1)
        .areas(area);
    area
}

/// Number of visible log rows inside the bordered overlay.
fn inner_height(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}

fn log_line(entry: &LogEntry) -> Line<'static> {
    Line::from(vec![
        Span::from(format!("{:>8.3}s ", entry.elapsed.as_secs_f64())).dark_gray(),
        Span::from(format!("{} ", entry.level.label())).fg(entry.level.color()),
        Span::from(format!("{} ", entry.target)).dark_gray(),
        Span::from(entry.message.clone()),
    ])
}

pub struct DebugLogModal {
    pub border_type: BorderType,
}

impl DebugLogModal {
    pub fn new(border_type: BorderType) -> Self {
        Self { border_type }
    }
}

impl StatefulWidget for DebugLogModal {
    type State = DebugLogModalState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = overlay_area(area);
        let page = inner_height(area);

        let entries = snapshot(state.min_level);
        let total = entries.len();

        state.offset = state.offset.min(total.saturating_sub(page));
        let end = total - state.offset;
        let start = end.saturating_sub(page);

        let lines: Vec<Line> = if entries.is_empty() {
            vec![Line::from(Span::from("No log entries").dark_gray())]
        } else {
            entries[start..end].iter().map(log_line).collect()
        };

        let title = format!(" Debug Log ({}+) ", state.min_level.label().trim());
        let block = Block::bordered()
            .dark_gray()
            .border_type(self.border_type)
            .padding(Padding::horizontal(1))
            .title_style(Style::default().italic().bold())
            .title(title)
            .title(Line::from(" (g<) ").alignment(Alignment::Right));

        Widget::render(Clear, area, buf);
        Widget::render(
            Paragraph::new(lines).block(block).fg(Color::default()),
            area,
            buf,
        );

        state.scrollbar_state = ScrollbarState::new(total).position(start);
        StatefulWidget::render(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            buf,
            &mut state.scrollbar_state,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    fn entry(level: LogLevel, message: &str) -> LogEntry {
        LogEntry {
            level,
            target: "basalt_tui::app".to_string(),
            message: message.to_string(),
            elapsed: Duration::from_millis(1234),
        }
    }

    #[test]
    fn level_from_tracing() {
        assert_eq!(LogLevel::from(tracing::Level::TRACE), LogLevel::Trace);
        assert_eq!(LogLevel::from(tracing::Level::ERROR), LogLevel::Error);
    }

    #[test]
    fn levels_are_ordered_by_severity() {
        assert!(LogLevel::Trace < LogLevel::Error);
        assert!(LogLevel::Info < LogLevel::Warn);
    }

    #[test]
    fn push_bounded_evicts_oldest() {
        let mut buffer = VecDeque::new();
        for index in 0..CAPACITY + 5 {
            push_bounded(
                &mut buffer,
                entry(LogLevel::Info, &format!("entry {index}")),
            );
        }
        assert_eq!(buffer.len(), CAPACITY);
        assert_eq!(buffer.front().unwrap().message, "entry 5");
        assert_eq!(
            buffer.back().unwrap().message,
            format!("entry {}", CAPACITY + 4)
        );
    }

    #[test]
    fn cycle_level_wraps_and_resets_offset() {
        let mut state = DebugLogModalState {
            offset: 7,
            ..Default::default()
        };
        state.cycle_level();
        assert_eq!(state.min_level, LogLevel::Debug);
        assert_eq!(state.offset, 0);
    }

    // Touches the process-global buffer, so it is the only global-state test.
    #[test]
    fn render_overlay() {
        clear();
        let entries = [
            entry(LogLevel::Trace, "entering run loop"),
            entry(LogLevel::Debug, "refreshed 142 entries"),
            entry(LogLevel::Info, "vault opened: Notes"),
            entry(LogLevel::Warn, "wiki link update failed"),
            entry(LogLevel::Error, "failed to create note"),
        ];
        if let Ok(mut buffer) = buffer().lock() {
            entries
                .into_iter()
                .for_each(|e| push_bounded(&mut buffer, e));
        }

        let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
        terminal
            .draw(|frame| {
                DebugLogModal::new(BorderType::Rounded).render(
                    frame.area(),
                    frame.buffer_mut(),
                    &mut DebugLogModalState {
                        visible: true,
                        ..Default::default()
                    },
                );
            })
            .unwrap();

        assert_snapshot!(terminal.backend());
        clear();
    }
}
