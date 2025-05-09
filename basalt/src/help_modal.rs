use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct HelpModalState {
    pub scrollbar_state: ScrollbarState,
    pub scrollbar_position: usize,
    pub viewport_height: usize,
    pub text: String,
    pub hidden: bool,
}

impl HelpModalState {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            scrollbar_state: ScrollbarState::new(text.lines().count()),
            ..Default::default()
        }
    }

    pub fn scroll_up(self, amount: usize) -> Self {
        let scrollbar_position = self.scrollbar_position.saturating_sub(amount);
        let scrollbar_state = self.scrollbar_state.position(scrollbar_position);

        Self {
            scrollbar_state,
            scrollbar_position,
            ..self
        }
    }

    pub fn scroll_down(self, amount: usize) -> Self {
        let scrollbar_position = self
            .scrollbar_position
            .saturating_add(amount)
            .min(self.text.lines().count());

        let scrollbar_state = self.scrollbar_state.position(scrollbar_position);

        Self {
            scrollbar_state,
            scrollbar_position,
            ..self
        }
    }

    pub fn reset_scrollbar(self) -> Self {
        Self {
            scrollbar_state: ScrollbarState::default(),
            scrollbar_position: 0,
            ..self
        }
    }
}

fn modal_area(area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(50)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(83)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

pub struct HelpModal;

impl StatefulWidget for HelpModal {
    type State = HelpModalState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let block = Block::bordered()
            .black()
            .border_type(BorderType::Rounded)
            .padding(Padding::uniform(1))
            .title_style(Style::default().italic().bold())
            .title(" Help ")
            .title(Line::from(" (?) ").alignment(Alignment::Right));

        let area = modal_area(area);

        Widget::render(Clear, area, buf);
        Widget::render(
            Paragraph::new(state.text.clone())
                .wrap(Wrap::default())
                .scroll((state.scrollbar_position as u16, 0))
                .block(block)
                .fg(Color::default()),
            area,
            buf,
        );

        StatefulWidget::render(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            buf,
            &mut state.scrollbar_state,
        );
    }
}
