use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        StatefulWidget, Widget, Wrap,
    },
};

mod state;

pub use state::HelpModalState;

pub struct Help {
    text: String,
}

impl Help {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }
}

impl StatefulWidget for Help {
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
            Paragraph::new(self.text)
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

fn modal_area(area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(50)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(83)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
