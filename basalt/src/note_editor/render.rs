use ratatui::{
    style::{Color, Modifier, Style, Stylize},
    text::{Span, ToSpan},
};

use unicode_width::UnicodeWidthStr;

use crate::{
    note_editor::{
        ast::{self, SourceRange},
        rich_text::RichText,
        text_wrap::wrap_preserve_trailing,
        virtual_document::{
            content_span, empty_virtual_line, synthetic_span, virtual_line, VirtualBlock,
            VirtualLine, VirtualSpan,
        },
    },
    stylized_text::{stylize, FontStyle},
};

trait SpanExt {
    fn merge(self, other: Span) -> Span;
}

impl SpanExt for &Span<'_> {
    fn merge(self, other: Span) -> Span {
        Span::styled(
            format!("{}{}", self.content, other.content),
            self.style.patch(other.style),
        )
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum RenderStyle {
    Raw,
    Visual,
}

// Internal consolidated text wrapping function
fn text_wrap_internal<'a>(
    text_content: &str,
    text_style: Style,
    prefix: Span<'static>,
    source_range: &SourceRange<usize>,
    width: usize,
    marker: Option<Span<'static>>,
    option: &RenderStyle,
) -> Vec<VirtualLine<'a>> {
    let prefix_width = prefix.width();

    let wrap_marker = "⤷ ";
    let wrapped_lines = wrap_preserve_trailing(text_content, width, wrap_marker.width() + 1);

    let mut current_range_start = source_range.start;

    wrapped_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_byte_len = line.width();

            let line_source_range =
                current_range_start..(current_range_start + line_byte_len).min(source_range.end);

            current_range_start += line_byte_len;

            let first_line = i == 0;
            let content_span = Span::styled(line.to_string(), text_style);

            match (&marker, first_line) {
                (Some(marker), true) if *option == RenderStyle::Visual => virtual_line!([
                    synthetic_span!(prefix),
                    synthetic_span!(marker),
                    content_span!(content_span, line_source_range)
                ]),
                (_, true) => virtual_line!([
                    synthetic_span!(prefix),
                    content_span!(content_span, line_source_range)
                ]),
                _ => {
                    virtual_line!([
                        synthetic_span!(prefix),
                        synthetic_span!(Span::styled(
                            " ".repeat(prefix_width.saturating_sub(1).max(1)),
                            prefix.style
                        )),
                        synthetic_span!(Span::styled(wrap_marker, Style::new().black())),
                        content_span!(content_span, line_source_range)
                    ])
                }
            }
        })
        .collect()
}

fn render_raw_line<'a>(
    line: &str,
    prefix: Span<'static>,
    source_range: &SourceRange<usize>,
    max_width: usize,
) -> Vec<VirtualLine<'a>> {
    text_wrap_internal(
        line,
        Style::default(),
        prefix,
        source_range,
        max_width,
        None,
        &RenderStyle::Raw,
    )
}

// # Example:
//
// | Basalt is a TUI (Terminal User Interface)
// |  ⤷ application to manage Obsidian vaults and
// |  ⤷ notes from the terminal. Basalt is
// |  ⤷ cross-platform and can be installed and run
// |  ⤷ in the major operating systems on Windows,
// |  ⤷ macOS; and Linux.
pub fn text_wrap<'a>(
    text: &Span<'a>,
    prefix: Span<'static>,
    source_range: &SourceRange<usize>,
    width: usize,
    marker: Option<Span<'static>>,
    option: &RenderStyle,
) -> Vec<VirtualLine<'a>> {
    text_wrap_internal(
        &text.content,
        text.style,
        prefix,
        source_range,
        width,
        marker,
        option,
    )
}

pub fn heading<'a>(
    level: ast::HeadingLevel,
    content: &str,
    prefix: Span<'static>,
    text: &RichText,
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
) -> VirtualBlock<'a> {
    use ast::HeadingLevel::*;
    // FIXME: Support new lines when editing
    // Currently when editing the heading and inserting new lines, the new lines are invisible and
    // only take affect visually when exiting (commiting edit changes)
    let text = text.to_string();
    let text = match option {
        RenderStyle::Visual => text,
        RenderStyle::Raw => content.to_string(),
    };

    let prefix_width = prefix.width();

    let h = |marker: Span<'static>, content: Span<'a>| {
        let mut wrapped_heading = text_wrap(
            &content,
            prefix.clone(),
            source_range,
            max_width,
            Some(marker),
            option,
        );

        wrapped_heading.push(empty_virtual_line!());
        wrapped_heading
    };

    let h_with_underline = |content: Span<'a>, underline: Span<'static>| {
        let mut wrapped_heading = text_wrap(
            &content,
            prefix.clone(),
            source_range,
            max_width,
            None,
            option,
        );
        wrapped_heading.push(virtual_line!([synthetic_span!(underline)]));
        wrapped_heading
    };

    let lines = match level {
        H1 => h_with_underline(
            if *option == RenderStyle::Visual {
                text.to_uppercase().bold()
            } else {
                text.bold()
            },
            "═".repeat(max_width.saturating_sub(prefix_width)).into(),
        ),
        H2 => h_with_underline(
            text.bold().yellow(),
            "─".repeat(max_width.saturating_sub(prefix_width)).yellow(),
        ),
        H3 => h("⬤  ".cyan(), text.bold().cyan()),
        H4 => h("● ".magenta(), text.bold().magenta()),
        H5 => h("◆ ".to_span(), stylize(&text, FontStyle::Script).into()),
        H6 => h("✺ ".to_span(), stylize(&text, FontStyle::Script).into()),
    };

    VirtualBlock::new(&lines, source_range)
}

pub fn render_raw<'a>(
    content: &str,
    source_range: &SourceRange<usize>,
    max_width: usize,
    prefix: Span<'static>,
) -> Vec<VirtualLine<'a>> {
    let mut current_range_start = source_range.start;

    let mut lines = content
        .lines()
        .flat_map(|line| {
            // TODO: Make sure that the line cannot exceed the source range end
            let line_range = line_range(current_range_start, line.width(), true);
            current_range_start = line_range.end;

            if line.is_empty() {
                vec![virtual_line!([
                    synthetic_span!(prefix.clone()),
                    content_span!("".to_string(), line_range)
                ])]
            } else {
                render_raw_line(line, prefix.clone(), &line_range, max_width)
            }
        })
        .collect::<Vec<_>>();

    lines.push(empty_virtual_line!());
    lines
}

pub fn paragraph<'a>(
    content: &str,
    prefix: Span<'static>,
    text: &RichText,
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix),
        RenderStyle::Visual => {
            let text = text.to_string();
            let mut current_range_start = source_range.start;

            let mut lines = text
                .to_string()
                .lines()
                .flat_map(|line| {
                    let line_range = line_range(current_range_start, line.width(), true);
                    current_range_start = line_range.end;

                    text_wrap(
                        &line.to_string().into(),
                        prefix.clone(),
                        &line_range,
                        max_width,
                        None,
                        option,
                    )
                })
                .collect::<Vec<_>>();

            if prefix.to_string().is_empty() {
                lines.extend([empty_virtual_line!()]);
            }

            lines
        }
    };

    VirtualBlock::new(&lines, source_range)
}

pub fn code_block<'a>(
    content: &str,
    prefix: Span<'static>,
    // TODO: Add lang support
    // Ref: https://github.com/erikjuhani/basalt/issues/96
    _lang: &Option<String>,
    text: &RichText,
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => {
            let mut current_range_start = source_range.start;

            let mut lines = content
                .lines()
                .map(|line| {
                    let line_range = line_range(current_range_start, line.width(), true);
                    current_range_start = line_range.end;

                    virtual_line!([
                        synthetic_span!(prefix.clone()),
                        synthetic_span!(Span::styled(" ", Style::new().bg(Color::Black))),
                        content_span!(line.to_string().bg(Color::Black), line_range),
                        synthetic_span!(" "
                            .repeat(
                                max_width
                                    .saturating_sub(prefix.width() + line.chars().count())
                                    .saturating_sub(1)
                            )
                            .bg(Color::Black)),
                    ])
                })
                .collect::<Vec<_>>();

            lines.push(empty_virtual_line!());
            lines
        }
        RenderStyle::Visual => {
            let text = text.to_string();

            let padding_line = virtual_line!([
                synthetic_span!(prefix.clone()),
                synthetic_span!(" "
                    .repeat(max_width.saturating_sub(prefix.width()))
                    .bg(Color::Black))
            ]);

            let mut current_range_start = source_range.start;

            let mut lines = vec![padding_line.clone()];
            lines.extend(text.lines().map(|line| {
                let line_byte_len = line.len();
                let source_range = current_range_start
                    ..(current_range_start + line_byte_len).min(source_range.end);
                current_range_start += line_byte_len;

                virtual_line!([
                    synthetic_span!(prefix.clone()),
                    synthetic_span!(Span::styled(" ", Style::new().bg(Color::Black))),
                    content_span!(line.to_string().bg(Color::Black), source_range),
                    synthetic_span!(" "
                        .repeat(
                            max_width
                                .saturating_sub(prefix.width() + line.chars().count())
                                .saturating_sub(1)
                        )
                        .bg(Color::Black)),
                ])
            }));
            lines.extend([padding_line]);
            lines.extend([empty_virtual_line!()]);
            lines
        }
    };

    VirtualBlock::new(&lines, source_range)
}

pub fn list<'a>(
    content: &str,
    prefix: Span<'static>,
    nodes: &[ast::Node],
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix),
        RenderStyle::Visual => {
            let mut lines: Vec<VirtualLine<'a>> = nodes
                .iter()
                .flat_map(|node| {
                    let node_content = content
                        .get(node.source_range().clone())
                        .unwrap_or("")
                        .to_string();
                    render_node(node_content, node, max_width, prefix.clone(), option).lines
                })
                .collect();

            if prefix.to_string().is_empty() {
                lines.extend([empty_virtual_line!()]);
            }
            lines
        }
    };

    VirtualBlock::new(&lines, source_range)
}

pub fn task<'a>(
    content: &str,
    prefix: Span<'static>,
    kind: &ast::TaskKind,
    nodes: &[ast::Node],
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix),
        RenderStyle::Visual => {
            let Some((text, rest)) = nodes.split_first().and_then(|(first, rest)| {
                let text = first.rich_text()?;
                Some((text, rest))
            }) else {
                return VirtualBlock::new(&[], source_range);
            };

            let text = text.to_string();
            let text = match option {
                RenderStyle::Visual => text,
                RenderStyle::Raw => content.to_string(),
            };
            let (marker, text) = match kind {
                ast::TaskKind::Unchecked => ("□ ".dark_gray(), text.into()),
                ast::TaskKind::LooselyChecked => ("■ ".magenta(), text.dark_gray()),
                ast::TaskKind::Checked => (
                    "■ ".magenta(),
                    text.dark_gray().add_modifier(Modifier::CROSSED_OUT),
                ),
            };

            let mut lines = text_wrap(
                &text,
                prefix.clone(),
                source_range,
                max_width,
                Some(marker),
                option,
            );

            lines.extend(rest.iter().flat_map(|node| {
                render_node(
                    content.to_string(),
                    node,
                    max_width,
                    prefix.merge("  ".into()),
                    option,
                )
                .lines
            }));

            lines
        }
    };

    VirtualBlock::new(&lines, source_range)
}

pub fn item<'a>(
    content: &str,
    prefix: Span<'static>,
    kind: &ast::ItemKind,
    nodes: &[ast::Node],
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix),
        RenderStyle::Visual => {
            let Some((text, rest)) = nodes.split_first().and_then(|(first, rest)| {
                let text = first.rich_text()?;
                Some((text, rest))
            }) else {
                return VirtualBlock::new(&[], source_range);
            };

            let text = text.to_string();

            let marker = match kind {
                ast::ItemKind::Ordered(i) => format!("{i}. ").dark_gray(),
                ast::ItemKind::Unordered => "- ".dark_gray(),
            };

            let mut lines = text_wrap(
                &text.into(),
                // TODO: Make the visual marker a separate prefix so we do not repeat it
                prefix.clone(),
                source_range,
                max_width,
                Some(marker),
                option,
            );

            lines.extend(rest.iter().flat_map(|node| {
                render_node(
                    content.to_string(),
                    node,
                    max_width,
                    prefix.merge("  ".into()),
                    option,
                )
                .lines
            }));

            lines
        }
    };

    VirtualBlock::new(&lines, source_range)
}

pub fn line_range(start: usize, line_width: usize, newline: bool) -> SourceRange<usize> {
    // NOTE: When the content is replaced by rope the new lines are kept
    // + 1 for newline
    let end = start + line_width + if newline { 1 } else { 0 };
    start..end
}

pub fn block_quote<'a>(
    content: &str,
    prefix: Span<'static>,
    // TODO: Add kind support
    // Should be as simple as adding a synthetic icon span and a content span
    // visual_line!([synthetic, content])
    // Ref: https://github.com/erikjuhani/basalt/issues/79
    _kind: &Option<ast::BlockQuoteKind>,
    nodes: &[ast::Node],
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix),
        RenderStyle::Visual => nodes
            .iter()
            .enumerate()
            .flat_map(|(i, node)| {
                let mut lines = render_node(
                    content.to_string(),
                    node,
                    max_width,
                    prefix.merge(Span::raw("┃ ").magenta()),
                    option,
                )
                .lines;
                if prefix.to_string().is_empty() && i != nodes.len().saturating_sub(1) {
                    lines.extend([virtual_line!([synthetic_span!(Span::raw("┃ ").magenta())])]);
                }
                if prefix.to_string().is_empty() && i == nodes.len().saturating_sub(1) {
                    lines.extend([empty_virtual_line!()]);
                }
                lines
            })
            .collect::<Vec<_>>(),
    };

    VirtualBlock::new(&lines, source_range)
}

pub fn render_node<'a>(
    content: String,
    node: &ast::Node,
    max_width: usize,
    prefix: Span<'static>,
    option: &RenderStyle,
) -> VirtualBlock<'a> {
    use ast::Node::*;
    match node {
        Heading {
            level,
            text,
            source_range,
        } => heading(
            *level,
            &content,
            prefix,
            text,
            source_range,
            max_width,
            option,
        ),
        Paragraph { text, source_range } => {
            paragraph(&content, prefix, text, source_range, max_width, option)
        }
        CodeBlock {
            lang,
            text,
            source_range,
        } => code_block(
            &content,
            prefix,
            lang,
            text,
            source_range,
            max_width,
            option,
        ),
        List {
            nodes,
            source_range,
        } => list(&content, prefix, nodes, source_range, max_width, option),
        Item {
            kind,
            nodes,
            source_range,
        } => item(
            &content,
            prefix,
            kind,
            nodes,
            source_range,
            max_width,
            option,
        ),
        Task {
            kind,
            nodes,
            source_range,
        } => task(
            &content,
            prefix,
            kind,
            nodes,
            source_range,
            max_width,
            option,
        ),
        BlockQuote {
            kind,
            nodes,
            source_range,
        } => block_quote(
            &content,
            prefix,
            kind,
            nodes,
            source_range,
            max_width,
            option,
        ),
    }
}
