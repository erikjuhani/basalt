//! # Markdown View Widget
//!
//! This module provides a widget called `MarkdownView` that can render Markdown content into
//! terminal user interface (TUI) structures using the [`ratatui`](https://docs.rs/ratatui) crate.
//! It integrates with a [`super::state::MarkdownViewState`] to manage scrolling and additional
//! metadata.
//!
//! The module uses markdown parser [`basalt_core::markdown`] to produce
//! [`basalt_core::markdown::Node`] values. Each node is converted to one or more
//! [`ratatui::text::Line`] objects.
//!
//! Example of rendered output
//!
//! ██ Headings
//!
//! █ This is a heading 1
//!
//! ██ This is a heading 2
//!
//! ▓▓▓ This is a heading 3
//!
//! ▓▓▓▓ This is a heading 4
//!
//! ▓▓▓▓▓ This is a heading 5
//!
//! ░░░░░░ This is a heading 6
//!
//! ██ Quotes
//!
//! You can quote text by adding a > symbols before the text.
//!
//! ┃ Human beings face ever more complex and urgent problems, and their effectiveness in dealing with these problems is a matter that is critical to the stability and continued progress of society.
//! ┃
//! ┃ - Doug Engelbart, 1961
//!
//! ██ Bold, italics, highlights
//!
//! This line will not be bold
//!
//! \*\*This line will not be bold\*\*
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Stylize},
    text::{Line, Span},
    widgets::{
        self, Block, BorderType, Paragraph, ScrollbarOrientation, StatefulWidget,
        StatefulWidgetRef, Widget,
    },
};

use basalt_core::markdown::{self, HeadingLevel, ItemKind};

use super::state::MarkdownViewState;

/// A widget for rendering markdown text using [`MarkdownViewState`].
///
/// # Example
///
/// ```rust
/// use basalt_core::markdown;
/// use basalt_widgets::markdown::{MarkdownViewState, MarkdownView};
/// use ratatui::prelude::*;
/// use ratatui::widgets::StatefulWidgetRef;
///
/// let text = "# Hello, world!\nThis is a test.";
/// let mut state = MarkdownViewState::new(text);
///
/// let area = Rect::new(0, 0, 20, 10);
/// let mut buffer = Buffer::empty(area);
///
/// MarkdownView.render_ref(area, &mut buffer, &mut state);
///
/// let expected = [
///   "╭──────────────────▲",
///   "│█ Hello, world!   █",
///   "│                  █",
///   "│This is a test.   █",
///   "│                  █",
///   "│                  █",
///   "│                  █",
///   "│                  ║",
///   "│                  ║",
///   "╰──────────────────▼",
/// ];
///
/// // FIXME: Take styles into account
/// // assert_eq!(buffer, Buffer::with_lines(expected));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownView;

impl MarkdownView {
    fn heading(level: HeadingLevel, content: Vec<Span>) -> Line {
        let prefix = match level {
            HeadingLevel::H1 => Span::from("█ ").blue(),
            HeadingLevel::H2 => Span::from("██ ").cyan(),
            HeadingLevel::H3 => Span::from("▓▓▓ ").green(),
            HeadingLevel::H4 => Span::from("▓▓▓▓ ").yellow(),
            HeadingLevel::H5 => Span::from("▓▓▓▓▓ ").red(),
            HeadingLevel::H6 => Span::from("░░░░░░ ").red(),
        };
        Line::from([prefix].into_iter().chain(content).collect::<Vec<_>>()).bold()
    }

    fn item<'a>(kind: Option<ItemKind>, content: Vec<Span<'a>>, prefix: Span<'a>) -> Line<'a> {
        match kind {
            Some(kind) => match kind {
                ItemKind::Unchecked => Line::from(
                    [prefix, "󰄱 ".black()]
                        .into_iter()
                        .chain(content)
                        .collect::<Vec<_>>(),
                ),
                ItemKind::LooselyChecked(_) => Line::from(
                    [prefix, "󰄲 ".magenta()]
                        .into_iter()
                        .chain(content)
                        .collect::<Vec<_>>(),
                ),
                ItemKind::HardChecked => Line::from(
                    [prefix, "󰄲 ".magenta()]
                        .into_iter()
                        .chain(content)
                        .collect::<Vec<_>>(),
                )
                .black()
                .add_modifier(Modifier::CROSSED_OUT),
                ItemKind::Ordered(num) => Line::from(
                    [prefix, num.to_string().black(), ". ".into()]
                        .into_iter()
                        .chain(content)
                        .collect::<Vec<_>>(),
                ),
                ItemKind::Unordered => Line::from(
                    [prefix, "- ".black()]
                        .into_iter()
                        .chain(content)
                        .collect::<Vec<_>>(),
                ),
                // ITS Theme markers — icon/color rendering added in 05-03
                _ => Line::from(
                    [prefix, "󰄱 ".dark_gray()]
                        .into_iter()
                        .chain(content)
                        .collect::<Vec<_>>(),
                ),
            },
            None => Line::from(
                [prefix, "- ".black()]
                    .into_iter()
                    .chain(content)
                    .collect::<Vec<_>>(),
            ),
        }
    }

    fn text_to_spans<'a>(text: markdown::Text) -> Vec<Span<'a>> {
        text.into_iter()
            .map(|text| Span::from(text.content))
            .collect()
    }

    fn code_block<'a>(text: markdown::Text) -> Vec<Line<'a>> {
        text.into_iter()
            .flat_map(|text| {
                text.content
                    .clone()
                    .split("\n")
                    .map(String::from)
                    .collect::<Vec<String>>()
            })
            .map(|text| Line::from(text).red().bg(Color::Rgb(10, 10, 10)))
            .collect()
    }

    fn render_markdown<'a>(node: markdown::Node, prefix: Span<'a>) -> Vec<Line<'a>> {
        match node.markdown_node {
            markdown::MarkdownNode::Paragraph { text } => {
                let mut spans = MarkdownView::text_to_spans(text);
                spans.insert(0, prefix.clone());
                vec![spans.into(), Line::from(prefix)]
            }
            markdown::MarkdownNode::Heading { level, text } => [
                MarkdownView::heading(level, MarkdownView::text_to_spans(text)),
                Line::default(),
            ]
            .to_vec(),
            markdown::MarkdownNode::Item { kind, text } => [
                MarkdownView::item(kind, MarkdownView::text_to_spans(text), prefix),
                Line::default(),
            ]
            .to_vec(),
            // TODO: Add lang support and syntax highlighting
            markdown::MarkdownNode::CodeBlock { text, .. } => {
                let mut lines = MarkdownView::code_block(text);
                lines.insert(0, Line::default());
                lines
            }
            // TODO: Support callout block quote types
            markdown::MarkdownNode::BlockQuote { nodes, .. } => {
                let mut lines = nodes
                    .into_iter()
                    .flat_map(|child| {
                        MarkdownView::render_markdown(child, Span::from("┃ ").magenta())
                    })
                    .map(|line| line.dark_gray())
                    .collect::<Vec<Line<'a>>>();

                lines.push(Line::default());

                lines
            }
            // Table rendering is implemented in plan 06-02; return empty for now.
            markdown::MarkdownNode::Table { .. } => vec![],
        }
    }
}

impl StatefulWidgetRef for MarkdownView {
    type State = MarkdownViewState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let nodes = markdown::from_str(&state.text)
            .into_iter()
            .flat_map(|node| MarkdownView::render_markdown(node, Span::default()))
            .collect::<Vec<Line<'_>>>();

        let mut scroll_state = state.scrollbar.state.content_length(nodes.len());

        let root_node = Paragraph::new(nodes)
            .block(Block::bordered().border_type(BorderType::Rounded))
            .scroll((state.scrollbar.position as u16, 0));

        Widget::render(root_node, area, buf);

        StatefulWidget::render(
            widgets::Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            buf,
            &mut scroll_state,
        );
    }
}
