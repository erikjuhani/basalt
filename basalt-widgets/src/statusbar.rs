use std::marker::PhantomData;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Stylize},
    text::{Line, Span, Text},
    widgets::{StatefulWidgetRef, Widget},
};

mod state;
pub use state::StatusBarState;

#[derive(Default)]
pub struct StatusBar<'a> {
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> StatefulWidgetRef for StatusBar<'a> {
    type State = StatusBarState<'a>;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [left, right] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(28)])
            .flex(Flex::SpaceBetween)
            .areas(area);

        let meta = state
            .meta
            .map(|meta| {
                [
                    Span::from(" ").bg(Color::Black),
                    Span::from(meta).bg(Color::Black).gray().bold(),
                    Span::from(" ").bg(Color::Black),
                    Span::from("").black(),
                ]
            })
            .unwrap_or_default();

        Text::from(Line::from(
            [
                Span::from("").magenta(),
                Span::from(" ").bg(Color::Magenta),
                Span::from(state.mode).magenta().reversed().bold(),
                Span::from(" ").bg(Color::Magenta),
                Span::from("")
                    .bg(if state.meta.is_some() {
                        Color::Black
                    } else {
                        Color::default()
                    })
                    .magenta(),
            ]
            .into_iter()
            .chain(meta)
            .collect::<Vec<Span>>(),
        ))
        .render(left, buf);

        let [word_count, char_count] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
                .flex(Flex::End)
                .areas(right);

        Text::from(format!("{} words", state.word_count))
            .right_aligned()
            .render(word_count, buf);

        Text::from(format!("{} chars", state.char_count))
            .right_aligned()
            .render(char_count, buf);
    }
}
