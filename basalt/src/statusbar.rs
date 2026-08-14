use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, StatefulWidget, Widget},
};

use crate::{config::Theme, note_editor::state::Mode};

/// Perceived brightness of an RGB colour (0..255). `None` for terminal-default
/// or ANSI colours, whose real value the app can't know.
fn luminance(color: Color) -> Option<f32> {
    match color {
        Color::Rgb(r, g, b) => Some(0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32),
        _ => None,
    }
}

/// The more legible of the theme's background / text over a filled `fill`, so a
/// mode block stays readable whatever colour the mode uses. Falls back to dark
/// text for ANSI/terminal-default themes, whose mode fills are light by default.
fn legible_over(fill: Color, theme: &Theme) -> Color {
    match (
        luminance(fill),
        luminance(theme.background),
        luminance(theme.text),
    ) {
        (Some(fill), Some(background), Some(text)) => {
            if (fill - text).abs() > (fill - background).abs() {
                theme.text
            } else {
                theme.background
            }
        }
        _ => Color::Black,
    }
}

#[derive(Default, Clone, PartialEq)]
pub struct StatusBarState<'a> {
    active_component_name: &'a str,
    mode: Mode,
    word_count: usize,
    char_count: usize,
}

impl<'a> StatusBarState<'a> {
    pub fn new(
        active_component_name: &'a str,
        mode: Mode,
        word_count: usize,
        char_count: usize,
    ) -> Self {
        Self {
            active_component_name,
            mode,
            word_count,
            char_count,
        }
    }
}

pub struct StatusBar<'a> {
    theme: &'a Theme,
}

impl<'a> StatusBar<'a> {
    pub(crate) fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }
}

impl<'a> StatefulWidget for StatusBar<'a> {
    type State = StatusBarState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let bar = self.theme.status_bar;
        Block::new()
            .style(Style::new().bg(bar.background))
            .render(area, buf);

        let [left, right] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(28)])
            .flex(Flex::SpaceBetween)
            .areas(area);

        // A rectangular mode block, then the active pane name.
        let mode = state.mode;
        let mode_color = mode.color(self.theme);
        let status = Line::from(vec![
            Span::from(format!(" {} ", mode.label()))
                .fg(legible_over(mode_color, self.theme))
                .bg(mode_color)
                .bold(),
            Span::from(format!("  {}", state.active_component_name))
                .fg(bar.foreground)
                .bold(),
        ]);
        Text::from(status).render(left, buf);

        let [word_count, char_count] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
                .flex(Flex::End)
                .areas(right);

        Text::from(
            format!(
                "{} word{}",
                state.word_count,
                if state.word_count == 1 { "" } else { "s" }
            )
            .fg(bar.foreground),
        )
        .right_aligned()
        .render(word_count, buf);

        Text::from(
            format!(
                "{} char{}",
                state.char_count,
                if state.char_count == 1 { "" } else { "s" }
            )
            .fg(bar.foreground),
        )
        .right_aligned()
        .render(char_count, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legible_over_picks_the_contrasting_neutral() {
        let theme = Theme {
            background: Color::Rgb(20, 20, 20),
            text: Color::Rgb(230, 230, 230),
            ..Theme::default()
        };
        // Light fill -> dark background text; dark fill -> light text.
        assert_eq!(
            legible_over(Color::Rgb(220, 210, 120), &theme),
            theme.background
        );
        assert_eq!(legible_over(Color::Rgb(60, 40, 40), &theme), theme.text);
    }

    #[test]
    fn legible_over_falls_back_to_dark_for_ansi_colours() {
        // Terminal-default themes have no known RGB; their mode fills are light,
        // so dark text stays legible.
        assert_eq!(legible_over(Color::Red, &Theme::default()), Color::Black);
    }
}
