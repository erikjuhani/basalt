use std::marker::PhantomData;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Stylize},
    text::{Line, Span, Text},
    widgets::{StatefulWidget, Widget},
};

#[derive(Default, Clone, PartialEq)]
pub struct StatusBarState<'a> {
    active_component_name: &'a str,
    word_count: usize,
    char_count: usize,
}

impl<'a> StatusBarState<'a> {
    pub fn new(active_component_name: &'a str, word_count: usize, char_count: usize) -> Self {
        Self {
            active_component_name,
            word_count,
            char_count,
        }
    }
}

#[derive(Default)]
pub struct StatusBar<'a> {
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> StatefulWidget for StatusBar<'a> {
    type State = StatusBarState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [left, right] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(28)])
            .flex(Flex::SpaceBetween)
            .areas(area);

        let active_component = [
            Span::from("").dark_gray(),
            Span::from(" ").bg(Color::DarkGray),
            Span::from(state.active_component_name)
                .dark_gray()
                .reversed()
                .bold(),
            Span::from(" ").bg(Color::DarkGray),
            Span::from("").dark_gray(),
        ]
        .to_vec();

        Text::from(Line::from(active_component)).render(left, buf);

        let [word_count, char_count] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
                .flex(Flex::End)
                .areas(right);

        Text::from(format!(
            "{} word{}",
            state.word_count,
            if state.word_count == 1 { "" } else { "s" }
        ))
        .right_aligned()
        .render(word_count, buf);

        Text::from(format!(
            "{} char{}",
            state.char_count,
            if state.char_count == 1 { "" } else { "s" }
        ))
        .right_aligned()
        .render(char_count, buf);
    }
}
