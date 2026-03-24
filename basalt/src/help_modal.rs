use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect, Size},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};

use crate::app::{calc_scroll_amount, Message as AppMessage, ScrollAmount};

fn modal_area_height(size: Size) -> usize {
    let vertical = Layout::vertical([Constraint::Percentage(50)]).flex(Flex::Center);
    let [area] = vertical.areas(Rect::new(0, 0, size.width, size.height.saturating_sub(3)));
    area.height.into()
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Toggle,
    Close,
    ScrollUp(ScrollAmount),
    ScrollDown(ScrollAmount),
}

pub fn update<'a>(
    message: &Message,
    screen_size: Size,
    state: &mut HelpModalState,
) -> Option<AppMessage<'a>> {
    match message {
        Message::Toggle => state.toggle_visibility(),
        Message::Close => state.hide(),
        Message::ScrollDown(scroll_amount) => {
            state.scroll_down(calc_scroll_amount(
                scroll_amount,
                modal_area_height(screen_size),
            ));
        }
        Message::ScrollUp(scroll_amount) => {
            state.scroll_up(calc_scroll_amount(
                scroll_amount,
                modal_area_height(screen_size),
            ));
        }
    };

    None
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct HelpModalState {
    pub scrollbar_state: ScrollbarState,
    pub scrollbar_position: usize,
    pub text: String,
    pub visible: bool,
}

impl HelpModalState {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            scrollbar_state: ScrollbarState::new(text.lines().count()),
            ..Default::default()
        }
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let scrollbar_position = self.scrollbar_position.saturating_sub(amount);
        let scrollbar_state = self.scrollbar_state.position(scrollbar_position);

        self.scrollbar_state = scrollbar_state;
        self.scrollbar_position = scrollbar_position;
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let scrollbar_position = self
            .scrollbar_position
            .saturating_add(amount)
            .min(self.text.lines().count());

        let scrollbar_state = self.scrollbar_state.position(scrollbar_position);

        self.scrollbar_state = scrollbar_state;
        self.scrollbar_position = scrollbar_position;
    }
}

fn modal_area(area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(50)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(83)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

pub struct HelpModal {
    pub border_type: BorderType,
}

impl HelpModal {
    pub fn new(border_type: BorderType) -> Self {
        Self { border_type }
    }
}

impl StatefulWidget for HelpModal {
    type State = HelpModalState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let block = Block::bordered()
            .dark_gray()
            .border_type(self.border_type)
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
