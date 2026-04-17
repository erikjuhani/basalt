use ratatui::{
    style::{Color, Modifier, Style, Stylize},
    text::Span,
};

use unicode_width::UnicodeWidthStr;

use crate::{
    config::Symbols,
    note_editor::{
        ast::{self, SourceRange},
        rich_text::{self, RichText},
        text_wrap::wrap_preserve_trailing,
        virtual_document::{
            content_span, empty_virtual_line, synthetic_span, virtual_line, VirtualBlock,
            VirtualLine, VirtualSpan,
        },
    },
    stylized_text::stylize,
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
// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
fn text_wrap_internal<'a>(
    text_content: &str,
    text_style: Style,
    prefix: Span<'static>,
    source_range: &SourceRange<usize>,
    width: usize,
    marker: Option<Span<'static>>,
    option: &RenderStyle,
    symbols: &Symbols,
) -> Vec<VirtualLine<'a>> {
    let wrap_marker = &symbols.wrap_marker;
    let wrapped_lines = wrap_preserve_trailing(text_content, width, wrap_marker.width() + 1);

    let mut current_range_start = source_range.start;

    wrapped_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_byte_len = line.len();

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
                    let marker_padding = marker.as_ref().map(|m| m.width()).unwrap_or(0);
                    virtual_line!([
                        synthetic_span!(prefix),
                        synthetic_span!(Span::styled(" ".repeat(marker_padding), prefix.style)),
                        synthetic_span!(Span::styled(wrap_marker.clone(), Style::new().black())),
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
    symbols: &Symbols,
) -> Vec<VirtualLine<'a>> {
    text_wrap_internal(
        // TODO: Replace with `»` as a synthetic symbol for tabs
        // Tab characters need to be replaced to spaces or other characters as the tab characters
        // will break the UI. Similarly the same issue that I was facing was solved by replacing
        // the tab characters: https://github.com/ratatui/ratatui/issues/1606#issuecomment-3172769529
        &line.replace("\t", "  "),
        Style::default(),
        prefix,
        source_range,
        max_width,
        None,
        &RenderStyle::Raw,
        symbols,
    )
}

/// Convert a `RichText` into a list of ratatui `Span`s, one per `TextSegment`,
/// each carrying the appropriate ratatui style for that segment's `rich_text::Style`.
fn rich_text_to_spans(text: &rich_text::RichText) -> Vec<Span<'static>> {
    text.segments()
        .iter()
        .map(|seg| {
            let span = Span::raw(seg.content.clone());
            match &seg.style {
                Some(rich_text::Style::Strong) => span.bold(),
                Some(rich_text::Style::Emphasis) => span.italic(),
                Some(rich_text::Style::Strikethrough) => span.add_modifier(Modifier::CROSSED_OUT),
                Some(rich_text::Style::Code) => span.fg(Color::Yellow).bg(Color::Rgb(30, 30, 30)),
                Some(rich_text::Style::InlineMath) => span.fg(Color::Magenta).italic(),
                None => span,
            }
        })
        .collect()
}

/// Word-wrap a slice of styled `Span`s to `max_width` columns, preserving per-span
/// styles. Each display line is a `Vec<Span<'static>>`.
///
/// This mirrors the wrapping logic in `text_wrap_internal` by concatenating span contents
/// to obtain the plain-text wrap result, then re-applying styles via byte-offset mapping.
fn wrap_styled_spans(
    spans: &[Span<'static>],
    max_width: usize,
    wrap_marker: &str,
) -> Vec<Vec<Span<'static>>> {
    if spans.is_empty() {
        return vec![vec![]];
    }

    // Build flat plain text and record the (start_byte, end_byte, Style) for each span.
    let mut plain = String::new();
    let mut span_ranges: Vec<(usize, usize, ratatui::style::Style)> = Vec::new();
    for span in spans {
        let start = plain.len();
        plain.push_str(&span.content);
        span_ranges.push((start, plain.len(), span.style));
    }

    // Wrap the plain text using the same function as `text_wrap_internal`.
    let wrap_marker_display_width = UnicodeWidthStr::width(wrap_marker);
    let wrapped = wrap_preserve_trailing(&plain, max_width, wrap_marker_display_width + 1);

    if wrapped.is_empty() {
        return vec![vec![]];
    }

    // Map each display line back to styled spans.
    let mut result: Vec<Vec<Span<'static>>> = Vec::new();
    let mut byte_cursor: usize = 0;
    let mut span_iter_idx: usize = 0;

    for line_str in &wrapped {
        let line_bytes = line_str.len();
        let line_end = byte_cursor + line_bytes;
        let mut line_spans: Vec<Span<'static>> = Vec::new();

        let mut pos = byte_cursor;
        while pos < line_end && span_iter_idx < span_ranges.len() {
            let (span_start, span_end, style) = span_ranges[span_iter_idx];
            if span_end <= pos {
                span_iter_idx += 1;
                continue;
            }
            let chunk_start = pos.max(span_start);
            let chunk_end = line_end.min(span_end);
            if chunk_start < chunk_end {
                if let Some(chunk) = plain.get(chunk_start..chunk_end) {
                    if !chunk.is_empty() {
                        line_spans.push(Span::styled(chunk.to_string(), style));
                    }
                }
                pos = chunk_end;
                if chunk_end >= span_end {
                    span_iter_idx += 1;
                }
            } else {
                break;
            }
        }

        result.push(line_spans);
        byte_cursor = line_end;
    }

    if result.is_empty() {
        result.push(vec![]);
    }

    result
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
    symbols: &Symbols,
) -> Vec<VirtualLine<'a>> {
    text_wrap_internal(
        &text.content,
        text.style,
        prefix,
        source_range,
        width,
        marker,
        option,
        symbols,
    )
}

// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
pub fn heading<'a>(
    level: ast::HeadingLevel,
    content: &str,
    prefix: Span<'static>,
    text: &RichText,
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
    symbols: &Symbols,
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
            symbols,
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
            symbols,
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
            symbols
                .h1_underline
                .repeat(max_width.saturating_sub(prefix_width))
                .into(),
        ),
        H2 => h_with_underline(
            text.bold().yellow(),
            symbols
                .h2_underline
                .repeat(max_width.saturating_sub(prefix_width))
                .yellow(),
        ),
        H3 => h(format!("{} ", symbols.h3_marker).cyan(), text.bold().cyan()),
        H4 => h(
            format!("{} ", symbols.h4_marker).magenta(),
            text.bold().magenta(),
        ),
        H5 => h(
            format!("{} ", symbols.h5_marker).into(),
            match symbols.h5_font_style {
                Some(style) => stylize(&text, style).into(),
                None => text.into(),
            },
        ),
        H6 => h(
            format!("{} ", symbols.h6_marker).into(),
            match symbols.h6_font_style {
                Some(style) => stylize(&text, style).into(),
                None => text.into(),
            },
        ),
    };

    VirtualBlock::new(&lines, source_range)
}

pub fn render_raw<'a>(
    content: &str,
    source_range: &SourceRange<usize>,
    max_width: usize,
    prefix: Span<'static>,
    symbols: &Symbols,
) -> Vec<VirtualLine<'a>> {
    let mut current_range_start = source_range.start;

    let mut lines = content
        .lines()
        .flat_map(|line| {
            // TODO: Make sure that the line cannot exceed the source range end
            let line_range = line_range(current_range_start, line.len(), true);
            current_range_start = line_range.end;

            if line.is_empty() {
                vec![virtual_line!([
                    synthetic_span!(prefix.clone()),
                    content_span!("".to_string(), line_range)
                ])]
            } else {
                render_raw_line(line, prefix.clone(), &line_range, max_width, symbols)
            }
        })
        .collect::<Vec<_>>();

    // When content is empty (e.g. empty file), produce a content line so the
    // cursor has something to land on.
    if lines.is_empty() {
        lines.push(virtual_line!([
            synthetic_span!(prefix),
            content_span!("".to_string(), source_range)
        ]));
    }

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
    symbols: &Symbols,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
        RenderStyle::Visual => {
            let styled_spans = rich_text_to_spans(text);
            let wrap_width = max_width;
            let wrap_marker = symbols.wrap_marker.clone();
            let mut current_range_start = source_range.start;
            let mut lines: Vec<VirtualLine<'a>> = Vec::new();

            // Collect logical lines by splitting on '\n' within segment content.
            // Each logical line is then word-wrapped using wrap_styled_spans which
            // preserves per-segment styles while reproducing the same line-break
            // positions as the original text_wrap_internal path.
            let mut logical_line: Vec<Span<'static>> = Vec::new();
            for span in &styled_spans {
                let content = span.content.as_ref();
                let parts: Vec<&str> = content.split('\n').collect();
                for (j, part) in parts.iter().enumerate() {
                    if !part.is_empty() {
                        logical_line.push(Span::styled(part.to_string(), span.style));
                    }
                    let is_last_part = j == parts.len() - 1;
                    if !is_last_part {
                        // Flush current logical line (explicit '\n' in source).
                        let display_lines =
                            wrap_styled_spans(&logical_line, wrap_width, &wrap_marker);
                        for (di, display_line_spans) in display_lines.iter().enumerate() {
                            let is_wrap_continuation = di > 0;
                            let line_byte_len: usize =
                                display_line_spans.iter().map(|s| s.content.len()).sum();
                            let line_range = current_range_start
                                ..(current_range_start + line_byte_len).min(source_range.end);
                            current_range_start += line_byte_len;

                            let mut vspans: Vec<VirtualSpan<'a>> =
                                vec![synthetic_span!(prefix.clone())];
                            if is_wrap_continuation {
                                vspans.push(synthetic_span!(Span::styled(
                                    wrap_marker.clone(),
                                    Style::new().black()
                                )));
                            }
                            for s in display_line_spans {
                                vspans.push(content_span!(s.clone(), line_range));
                            }
                            lines.push(VirtualLine::new(&vspans));
                        }
                        // +1 for the '\n' byte.
                        current_range_start += 1;
                        logical_line = Vec::new();
                    }
                }
            }

            // Flush the final (or only) logical line.
            {
                let display_lines = wrap_styled_spans(&logical_line, wrap_width, &wrap_marker);
                for (di, display_line_spans) in display_lines.iter().enumerate() {
                    let is_wrap_continuation = di > 0;
                    let line_byte_len: usize =
                        display_line_spans.iter().map(|s| s.content.len()).sum();
                    let line_range = current_range_start
                        ..(current_range_start + line_byte_len).min(source_range.end);
                    current_range_start += line_byte_len;

                    let mut vspans: Vec<VirtualSpan<'a>> = vec![synthetic_span!(prefix.clone())];
                    if is_wrap_continuation {
                        vspans.push(synthetic_span!(Span::styled(
                            wrap_marker.clone(),
                            Style::new().black()
                        )));
                    }
                    for s in display_line_spans {
                        vspans.push(content_span!(s.clone(), line_range));
                    }
                    lines.push(VirtualLine::new(&vspans));
                }
            }

            if lines.is_empty() {
                // Empty paragraph — push a line with just the prefix so cursor has a home.
                lines.push(VirtualLine::new(&[synthetic_span!(prefix.clone())]));
            }

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
                    let line_range = line_range(current_range_start, line.len(), true);
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
                let source_range = line_range(current_range_start, line.len(), true);
                current_range_start = source_range.end;

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

// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
pub fn list<'a>(
    content: &str,
    prefix: Span<'static>,
    nodes: &[ast::Node],
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
    symbols: &Symbols,
    list_depth: usize,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
        RenderStyle::Visual => {
            let mut lines: Vec<VirtualLine<'a>> = nodes
                .iter()
                .flat_map(|node| {
                    let node_content = content
                        .get(node.source_range().clone())
                        .unwrap_or("")
                        .to_string();
                    render_node(
                        node_content,
                        node,
                        max_width,
                        prefix.clone(),
                        option,
                        symbols,
                        list_depth,
                    )
                    .lines
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

// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
pub fn task<'a>(
    content: &str,
    prefix: Span<'static>,
    kind: &ast::TaskKind,
    nodes: &[ast::Node],
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
    symbols: &Symbols,
    list_depth: usize,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
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
                ast::TaskKind::Unchecked => (
                    format!("{} ", symbols.task_unchecked).dark_gray(),
                    text.into(),
                ),
                ast::TaskKind::LooselyChecked => (
                    format!("{} ", symbols.task_checked).magenta(),
                    text.dark_gray(),
                ),
                ast::TaskKind::Checked => (
                    format!("{} ", symbols.task_checked).magenta(),
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
                symbols,
            );

            lines.extend(rest.iter().flat_map(|node| {
                render_node(
                    content.to_string(),
                    node,
                    max_width,
                    prefix.merge("  ".into()),
                    option,
                    symbols,
                    list_depth + 1,
                )
                .lines
            }));

            lines
        }
    };

    VirtualBlock::new(&lines, source_range)
}

// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
pub fn item<'a>(
    content: &str,
    prefix: Span<'static>,
    kind: &ast::ItemKind,
    nodes: &[ast::Node],
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
    symbols: &Symbols,
    list_depth: usize,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
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
                ast::ItemKind::Unordered => {
                    let marker = if symbols.list_markers.is_empty() {
                        "-"
                    } else {
                        &symbols.list_markers[list_depth % symbols.list_markers.len()]
                    };
                    format!("{marker} ").dark_gray()
                }
            };

            let mut lines = text_wrap(
                &text.into(),
                // TODO: Make the visual marker a separate prefix so we do not repeat it
                prefix.clone(),
                source_range,
                max_width,
                Some(marker),
                option,
                symbols,
            );

            lines.extend(rest.iter().flat_map(|node| {
                render_node(
                    content.to_string(),
                    node,
                    max_width,
                    prefix.merge("  ".into()),
                    option,
                    symbols,
                    list_depth + 1,
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

/// Renders a horizontal rule as a full-width separator line using the
/// configured `symbols.horizontal_rule` character (default: "═").
fn rule<'a>(
    source_range: &SourceRange<usize>,
    max_width: usize,
    prefix: Span<'static>,
    symbols: &Symbols,
) -> VirtualBlock<'a> {
    let separator = symbols
        .horizontal_rule
        .repeat(max_width.saturating_sub(prefix.width()));
    let lines = vec![
        virtual_line!([
            synthetic_span!(prefix),
            synthetic_span!(Span::raw(separator))
        ]),
        empty_virtual_line!(),
    ];
    VirtualBlock::new(&lines, source_range)
}

/// Renders a display math block as a three-line layout:
/// separator line (thin "─"), formula content, separator line.
/// Separator width matches the formula text width (not full terminal width).
/// All lines styled in Magenta; formula is italic.
fn display_math<'a>(
    content: &str,
    source_range: &SourceRange<usize>,
    prefix: Span<'static>,
) -> VirtualBlock<'a> {
    let formula = content.trim();
    let formula_width = UnicodeWidthStr::width(formula);
    let separator = "\u{2500}".repeat(formula_width); // U+2500 BOX DRAWINGS LIGHT HORIZONTAL

    let sep_span = Span::styled(separator, Style::default().fg(Color::Magenta));
    let formula_span = Span::styled(
        formula.to_string(),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::ITALIC),
    );

    let lines = vec![
        virtual_line!([
            synthetic_span!(prefix.clone()),
            synthetic_span!(sep_span.clone())
        ]),
        virtual_line!([
            synthetic_span!(prefix.clone()),
            synthetic_span!(formula_span)
        ]),
        virtual_line!([
            synthetic_span!(prefix),
            synthetic_span!(sep_span)
        ]),
        empty_virtual_line!(),
    ];
    VirtualBlock::new(&lines, source_range)
}

// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
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
    symbols: &Symbols,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
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
                    symbols,
                    0,
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
    symbols: &Symbols,
    list_depth: usize,
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
            symbols,
        ),
        Paragraph { text, source_range } => paragraph(
            &content,
            prefix,
            text,
            source_range,
            max_width,
            option,
            symbols,
        ),
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
        } => list(
            &content,
            prefix,
            nodes,
            source_range,
            max_width,
            option,
            symbols,
            list_depth,
        ),
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
            symbols,
            list_depth,
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
            symbols,
            list_depth,
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
            symbols,
        ),
        Rule { source_range } => rule(source_range, max_width, prefix, symbols),
        DisplayMath {
            content,
            source_range,
        } => display_math(content, source_range, prefix),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note_editor::rich_text::{Style as RichStyle, TextSegment};

    #[test]
    fn test_rich_text_to_spans_inline_math() {
        let text = RichText::from(vec![TextSegment::styled("E = mc^2", RichStyle::InlineMath)]);
        let spans = rich_text_to_spans(&text);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "E = mc^2");
        let expected = ratatui::style::Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::ITALIC);
        assert_eq!(spans[0].style, expected);
    }

    #[test]
    fn test_rule_render() {
        use crate::config::Symbols;
        let symbols = Symbols::default();
        let source_range = 0..3;
        let max_width = 20;
        let prefix = Span::raw("");
        let block = rule(&source_range, max_width, prefix, &symbols);
        // Block should have 2 lines: separator + empty
        assert_eq!(block.lines.len(), 2);
        // First line should contain the repeated horizontal_rule character
        let first_line = &block.lines[0];
        let content: String = first_line
            .virtual_spans()
            .iter()
            .map(|s| match s {
                VirtualSpan::Synthetic(span) | VirtualSpan::Content(span, _) => {
                    span.content.to_string()
                }
            })
            .collect();
        assert!(content.contains(&symbols.horizontal_rule.repeat(20)));
    }

    fn vline_content(line: &VirtualLine<'_>) -> String {
        line.virtual_spans()
            .iter()
            .map(|s| match s {
                VirtualSpan::Synthetic(span) | VirtualSpan::Content(span, _) => {
                    span.content.to_string()
                }
            })
            .collect()
    }

    #[test]
    fn test_display_math_render() {
        let content = "\n\\int_0^\\infty e^{-x} dx = 1\n";
        let source_range = 0..35;
        let prefix = Span::raw("");
        let block = display_math(content, &source_range, prefix);
        // Block should have 4 lines: separator + formula + separator + empty
        assert_eq!(block.lines.len(), 4);

        // First line (separator): contains "─" repeated to formula width
        let sep_line = vline_content(&block.lines[0]);
        assert!(sep_line.contains("\u{2500}"), "Separator should use U+2500");
        assert!(!sep_line.contains('═'), "Separator should NOT use horizontal_rule char");

        // Second line (formula): contains trimmed formula text
        let formula_line = vline_content(&block.lines[1]);
        assert!(formula_line.contains("\\int_0^\\infty"));
        assert!(!formula_line.starts_with('\n'), "Formula should be trimmed");

        // Third line (separator): same as first
        let sep_line_2 = vline_content(&block.lines[2]);
        assert_eq!(sep_line, sep_line_2, "Top and bottom separators should match");

        // Verify formula span has Magenta + ITALIC
        let formula_vspan = block.lines[1]
            .virtual_spans()
            .iter()
            .find(|s| match s {
                VirtualSpan::Synthetic(span) | VirtualSpan::Content(span, _) => {
                    span.content.contains("\\int")
                }
            })
            .expect("formula span not found");
        let formula_rspan = match formula_vspan {
            VirtualSpan::Synthetic(span) | VirtualSpan::Content(span, _) => span,
        };
        assert_eq!(formula_rspan.style.fg, Some(Color::Magenta));
        assert!(formula_rspan.style.add_modifier.contains(Modifier::ITALIC));
    }
}
