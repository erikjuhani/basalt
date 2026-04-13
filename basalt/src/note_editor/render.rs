use indexmap::IndexMap;
use ratatui::{
    style::{Color, Modifier, Style, Stylize},
    text::Span,
};

use unicode_width::UnicodeWidthStr;

use crate::{
    app::SyntectContext,
    config::Symbols,
    note_editor::{
        ast::{self, SourceRange},
        rich_text::{InlineNode, RichText},
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

/// Table border characters for the ASCII preset.
/// Layout: (top_left, top_mid, top_right, mid_left, mid_mid, mid_right,
///          bot_left, bot_mid, bot_right, h_line, v_line)
const TABLE_BORDERS_ASCII: (
    &str, &str, &str,
    &str, &str, &str,
    &str, &str, &str,
    &str, &str,
) = (
    "+", "+", "+",   // top: left, mid (column sep), right
    "+", "+", "+",   // header-body separator: left, mid, right
    "+", "+", "+",   // bottom: left, mid, right
    "-",             // horizontal line char
    "|",             // vertical line char
);

/// Table border characters for Unicode/NerdFont presets.
const TABLE_BORDERS_UNICODE: (
    &str, &str, &str,
    &str, &str, &str,
    &str, &str, &str,
    &str, &str,
) = (
    "┌", "┬", "┐",
    "├", "┼", "┤",
    "└", "┴", "┘",
    "─",
    "│",
);

fn table_borders(
    preset: &crate::config::Preset,
) -> (
    &'static str, &'static str, &'static str,
    &'static str, &'static str, &'static str,
    &'static str, &'static str, &'static str,
    &'static str, &'static str,
) {
    use crate::config::Preset;
    match preset {
        Preset::Ascii => TABLE_BORDERS_ASCII,
        _ => TABLE_BORDERS_UNICODE, // Unicode and NerdFont both use box-drawing
    }
}

fn rich_text_display_width(rt: &RichText) -> usize {
    let s: String = rt
        .nodes()
        .iter()
        .map(|node| match node {
            InlineNode::Text(seg) => seg.to_string(),
            InlineNode::Link { text, .. } => text.clone(),
            InlineNode::FootnoteRef(label) => format!("[{}]", label),
        })
        .collect();
    s.width().max(1) // minimum width 1 so empty cells have a column
}

/// Convert a [`RichText`] into styled [`Span`]s, applying appropriate ratatui styles.
fn rich_text_to_spans<'a>(rt: &RichText) -> Vec<Span<'a>> {
    rt.nodes()
        .iter()
        .map(|node| match node {
            InlineNode::Text(seg) => {
                let style = match &seg.style {
                    Some(crate::note_editor::rich_text::Style::Strong) => {
                        Style::default().add_modifier(Modifier::BOLD)
                    }
                    Some(crate::note_editor::rich_text::Style::Emphasis) => {
                        Style::default().add_modifier(Modifier::ITALIC)
                    }
                    Some(crate::note_editor::rich_text::Style::Strikethrough) => {
                        Style::default().add_modifier(Modifier::CROSSED_OUT)
                    }
                    Some(crate::note_editor::rich_text::Style::Code) => {
                        Style::default().fg(Color::Yellow)
                    }
                    Some(crate::note_editor::rich_text::Style::InlineMath) => {
                        Style::default().fg(Color::Cyan)
                    }
                    None => Style::default(),
                };
                Span::styled(seg.content.clone(), style)
            }
            InlineNode::Link { text, .. } => {
                // LINK-01: underline + distinct color for hyperlinks
                Span::styled(
                    text.clone(),
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::UNDERLINED),
                )
            }
            InlineNode::FootnoteRef(label) => {
                // FOOT-01: [N] in distinct color for footnote references
                Span::styled(
                    format!("[{}]", label),
                    Style::default().fg(Color::LightCyan),
                )
            }
        })
        .collect()
}

/// Scroll indicators for horizontally scrolled tables (D-14, D-15).
const INDICATOR_LEFT: &str = "◀";
const INDICATOR_RIGHT: &str = "▶";

/// Clip a full row string to the visible window determined by `h_scroll` and `max_width`,
/// returning the visible slice as a `String`.
///
/// `h_scroll` is the number of display-width chars to skip from the left.
/// The result is at most `max_width` display-width chars wide.
fn clip_row_str(full: &str, h_scroll: usize, max_width: usize) -> String {
    let mut chars_out = String::new();
    let mut display_pos: usize = 0;
    let mut visible_w: usize = 0;
    for ch in full.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        let next_pos = display_pos + cw;
        if next_pos <= h_scroll {
            // Entirely before the window — skip
            display_pos = next_pos;
            continue;
        }
        if display_pos < h_scroll {
            // Partially before the window — replace with spaces to preserve alignment
            let visible_part = next_pos - h_scroll;
            let fill = " ".repeat(visible_part);
            if visible_w + visible_part > max_width {
                break;
            }
            chars_out.push_str(&fill);
            visible_w += visible_part;
            display_pos = next_pos;
            continue;
        }
        // Fully within or after the window
        if visible_w + cw > max_width {
            break;
        }
        chars_out.push(ch);
        visible_w += cw;
        display_pos = next_pos;
    }
    chars_out
}

#[allow(clippy::too_many_arguments)]
pub fn table<'a>(
    alignments: &[ast::Alignment],
    header: &[RichText],
    rows: &[Vec<RichText>],
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
    symbols: &Symbols,
    h_scroll: usize,
) -> VirtualBlock<'a> {
    if *option == RenderStyle::Raw {
        return VirtualBlock::new(&[], source_range);
    }

    // Pass 1 — compute column widths
    let n_cols = header
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    let col_widths: Vec<usize> = (0..n_cols)
        .map(|i| {
            let hdr_w = header.get(i).map(rich_text_display_width).unwrap_or(1);
            let body_w = rows
                .iter()
                .map(|row| row.get(i).map(rich_text_display_width).unwrap_or(1))
                .max()
                .unwrap_or(1);
            hdr_w.max(body_w)
        })
        .collect();

    // Total display width of the full table:
    // left_border(1) + for each col: h_line*(col_w+2) + separator(1) * (n-1) + right_border(1)
    // = 1 + sum(col_w + 2) + (n_cols - 1) + 1
    // = 2 + sum(col_w + 2) + (n_cols - 1)  [if n_cols > 0]
    let total_table_w: usize = if n_cols == 0 {
        2 // just left + right border
    } else {
        1 + col_widths.iter().map(|w| w + 2).sum::<usize>() + (n_cols - 1) + 1
    };

    let show_left = h_scroll > 0;
    let show_right = h_scroll + max_width < total_table_w;

    // Border selection
    let (tl, tm, tr, ml, mm, mr, bl, bm, br, h, v) = table_borders(&symbols.preset);

    // Build the full horizontal rule string (without clipping), then clip it.
    let build_h_rule_str = |left: &str, mid: &str, right: &str| -> String {
        let mut s = String::from(left);
        for (i, &w) in col_widths.iter().enumerate() {
            s.push_str(&h.repeat(w + 2));
            if i + 1 < col_widths.len() {
                s.push_str(mid);
            }
        }
        s.push_str(right);
        s
    };

    // Helper: apply scroll indicators to a mutable Vec<char> representation of a clipped row.
    // `show_left` → replace char at index 0 with ◀
    // `show_right` → replace last char with ▶
    let apply_indicators = |s: &str| -> String {
        let chars: Vec<char> = s.chars().collect();
        if chars.is_empty() {
            return s.to_string();
        }
        let mut out = chars.clone();
        if show_left && !out.is_empty() {
            // Replace first character (the left border) with ◀
            out[0] = INDICATOR_LEFT.chars().next().unwrap_or('◀');
        }
        if show_right && !out.is_empty() {
            // Replace last character (the right border) with ▶
            let last = out.len() - 1;
            out[last] = INDICATOR_RIGHT.chars().next().unwrap_or('▶');
        }
        out.iter().collect()
    };

    // Helper: build a clipped + indicator-applied horizontal rule line
    let h_rule = |left: &str, mid: &str, right: &str| -> VirtualLine {
        let full = build_h_rule_str(left, mid, right);
        let clipped = clip_row_str(&full, h_scroll, max_width);
        let with_ind = apply_indicators(&clipped);
        VirtualLine::new(&[synthetic_span!(Span::raw(with_ind))])
    };

    // Helper: build a cell row (header or body) with h_scroll clipping and indicators.
    // Strategy: build the full row as a string (for clipping), then emit as a single synthetic
    // span for the border/padding chars — cell content becomes part of the string.
    // This is acceptable since table cells are typically short and are already synthetic_span.
    let cell_row = |cells: &[RichText], bold: bool| -> VirtualLine {
        // Build full row string
        let mut full = String::from(v);
        for (i, cell) in cells.iter().enumerate() {
            let align = alignments.get(i).copied().unwrap_or(ast::Alignment::None);
            let cell_w = col_widths.get(i).copied().unwrap_or(1);
            let cell_str: String = cell.to_string();
            let display_w = cell_str.width();
            let pad = cell_w.saturating_sub(display_w);
            let (left_pad, right_pad) = match align {
                ast::Alignment::Right => (pad, 0),
                ast::Alignment::Center => {
                    let lp = pad / 2;
                    (lp, pad - lp)
                }
                _ => (0, pad), // Left / None
            };
            full.push(' ');
            full.push_str(&" ".repeat(left_pad));
            full.push_str(&cell_str);
            full.push_str(&" ".repeat(right_pad));
            full.push(' ');
            if i + 1 < cells.len() {
                full.push_str(v);
            }
        }
        // Handle rows with fewer cells than n_cols
        for i in cells.len()..n_cols {
            let cell_w = col_widths.get(i).copied().unwrap_or(1);
            full.push_str(v);
            full.push_str(&" ".repeat(cell_w + 2));
        }
        full.push_str(v);

        // Apply h_scroll clipping
        let clipped = clip_row_str(&full, h_scroll, max_width);
        let with_ind = apply_indicators(&clipped);

        // Emit as a single styled span (bold modifier for header)
        let span = if bold {
            Span::styled(with_ind, Style::default().add_modifier(Modifier::BOLD))
        } else {
            Span::raw(with_ind)
        };
        VirtualLine::new(&[synthetic_span!(span)])
    };

    // Assemble lines
    let mut lines = vec![
        h_rule(tl, tm, tr),      // top border
        cell_row(header, true),  // header (bold)
        h_rule(ml, mm, mr),      // header-body separator
    ];
    for row in rows {
        lines.push(cell_row(row, false));
    }
    lines.push(h_rule(bl, bm, br)); // bottom border

    VirtualBlock::new(&lines, source_range)
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
            let text = text.to_string();
            let mut current_range_start = source_range.start;

            let mut lines = text
                .to_string()
                .lines()
                .flat_map(|line| {
                    let line_range = line_range(current_range_start, line.len(), true);
                    current_range_start = line_range.end;

                    text_wrap(
                        &line.to_string().into(),
                        prefix.clone(),
                        &line_range,
                        max_width,
                        None,
                        option,
                        symbols,
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

#[allow(clippy::too_many_arguments)]
pub fn code_block<'a>(
    content: &str,
    prefix: Span<'static>,
    // TODO: Add lang support
    // Ref: https://github.com/erikjuhani/basalt/issues/96
    _lang: &Option<String>,
    _syntect_ctx: Option<&SyntectContext>,
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
                        None,
                        0, // nested list items are not the active table
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
                // Standard markers — use existing symbols fields (D-11)
                ast::TaskKind::Unchecked => (
                    format!("{} ", symbols.task_unchecked).dark_gray(),
                    text.into(),
                ),
                ast::TaskKind::Checked => (
                    format!("{} ", symbols.task_checked).magenta(),
                    text.dark_gray().add_modifier(Modifier::CROSSED_OUT),
                ),

                // Unknown marker — render original [char] in dark_gray (D-16)
                ast::TaskKind::LooselyChecked(c) => (
                    format!("[{}] ", c).dark_gray(),
                    text.into(),
                ),

                // ITS Theme markers — delegate to task_style() for icon + color (D-13, D-14, D-15)
                its_kind => {
                    let (color, ascii, unicode, nerdfont) = task_style(its_kind);
                    let icon = match symbols.preset {
                        crate::config::Preset::Ascii => ascii,
                        crate::config::Preset::NerdFont => nerdfont,
                        // Unicode and Auto both use unicode icons
                        _ => unicode,
                    };
                    (
                        format!("{} ", icon).fg(color),
                        text.into(), // ITS Theme markers do NOT strike-through text (D-15)
                    )
                }
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
                    None,
                    0, // task sub-items are not the active table
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
                    None,
                    0, // item sub-nodes are not the active table
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

// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
pub fn block_quote<'a>(
    content: &str,
    prefix: Span<'static>,
    kind: &Option<ast::BlockQuoteKind>,
    title: &Option<String>,
    nodes: &[ast::Node],
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
    symbols: &Symbols,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
        RenderStyle::Visual => {
            let mut lines = Vec::new();
            let border_color = kind.as_ref().map(|k| callout_style(k).0).unwrap_or(Color::Magenta);

            // Header line for callouts (D-05, D-07)
            if let Some(ref k) = kind {
                let (color, _, _, _) = callout_style(k);
                let icon = callout_icon(k, &symbols.preset);
                let type_name = callout_type_name(k);
                let header_text = match title {
                    Some(t) => format!("{} {}: {}", icon, type_name, t),
                    None => format!("{} {}", icon, type_name),
                };
                let header_span = Span::styled(header_text, Style::default().fg(color).add_modifier(Modifier::BOLD));
                lines.push(virtual_line!([synthetic_span!(prefix.clone()), synthetic_span!(header_span)]));
            }

            // Body lines with type-colored border
            let body_lines: Vec<_> = nodes
                .iter()
                .enumerate()
                .flat_map(|(i, node)| {
                    let border_span = Span::raw("┃ ").fg(border_color);
                    let mut node_lines = render_node(
                        content.to_string(),
                        node,
                        max_width,
                        prefix.merge(border_span),
                        option,
                        symbols,
                        0,
                        None,
                        0, // block_quote inner nodes are not the active table
                    )
                    .lines;
                    if prefix.to_string().is_empty() && i != nodes.len().saturating_sub(1) {
                        let sep_border = Span::raw("┃ ").fg(border_color);
                        node_lines.extend([virtual_line!([synthetic_span!(sep_border)])]);
                    }
                    if prefix.to_string().is_empty() && i == nodes.len().saturating_sub(1) {
                        node_lines.extend([empty_virtual_line!()]);
                    }
                    node_lines
                })
                .collect();
            lines.extend(body_lines);
            lines
        },
    };

    VirtualBlock::new(&lines, source_range)
}

// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
pub fn render_node<'a>(
    content: String,
    node: &ast::Node,
    max_width: usize,
    prefix: Span<'static>,
    option: &RenderStyle,
    symbols: &Symbols,
    list_depth: usize,
    syntect_ctx: Option<&SyntectContext>,
    // Horizontal scroll offset for the active table (D-15). Pass `state.table_h_scroll`
    // when this node is a table and horizontal scrolling should apply; `0` otherwise.
    table_h_scroll: usize,
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
            syntect_ctx,
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
            title,
            nodes,
            source_range,
        } => block_quote(
            &content,
            prefix,
            kind,
            title,
            nodes,
            source_range,
            max_width,
            option,
            symbols,
        ),
        Table {
            alignments,
            header,
            rows,
            source_range,
        } => table(
            alignments,
            header,
            rows,
            source_range,
            max_width,
            option,
            symbols,
            table_h_scroll,
        ),
        FootnoteSection { defs, source_range } => {
            render_footnote_section(defs, source_range, max_width, symbols)
        }
    }
}

/// Renders footnote definitions section with a separator line and [N]: definition entries.
fn render_footnote_section<'a>(
    defs: &IndexMap<String, RichText>,
    source_range: &SourceRange<usize>,
    max_width: usize,
    symbols: &Symbols,
) -> VirtualBlock<'a> {
    let mut lines: Vec<VirtualLine> = Vec::new();

    // Separator line — reuse horizontal_rule char from symbols
    let rule_char = &symbols.horizontal_rule;
    let rule_str = rule_char.repeat(max_width);
    lines.push(virtual_line!([synthetic_span!(Span::styled(
        rule_str,
        Style::default().fg(Color::DarkGray)
    ))]));

    // One line per footnote definition
    for (label, content) in defs {
        let prefix = format!("[{}]: ", label);
        let prefix_span = Span::styled(prefix, Style::default().fg(Color::DarkGray));
        let content_spans = rich_text_to_spans(content);

        let mut spans = vec![synthetic_span!(prefix_span)];
        spans.extend(content_spans.into_iter().map(|s| synthetic_span!(s)));
        lines.push(virtual_line!(spans));
    }

    // Empty line after section
    lines.push(empty_virtual_line!());

    VirtualBlock::new(&lines, source_range)
}

// Callout icon/color table - per-type styling
fn callout_style(kind: &ast::BlockQuoteKind) -> (Color, &'static str, &'static str, &'static str) {
    use ast::BlockQuoteKind::*;
    // (color, ascii_icon, unicode_icon, nerdfont_icon)
    match kind {
        // Standard GitHub Alert types (D-10)
        Note => (Color::Cyan, "[i]", "\u{2139}", "\u{f05a}"),
        Tip => (Color::Green, "[*]", "\u{2728}", "\u{f0eb}"),
        Important => (Color::Magenta, "[!]", "\u{2691}", "\u{f024}"),
        Warning => (Color::Yellow, "[!]", "\u{26a0}", "\u{f071}"),
        Caution => (Color::Red, "[x]", "\u{2716}", "\u{f00d}"),

        // ITS Theme - Citation/text (warm tones)
        Quote => (Color::Yellow, "[q]", "\u{275d}", "\u{f10d}"),
        Recite => (Color::LightYellow, "[r]", "\u{275e}", "\u{f10e}"),
        Aside => (Color::Cyan, "[>]", "\u{00bb}", "\u{f101}"),

        // ITS Theme - Structural/layout (cool tones)
        Cards => (Color::Blue, "[#]", "\u{25a3}", "\u{f0db}"),
        Grid => (Color::LightBlue, "[G]", "\u{25a6}", "\u{f00a}"),
        Column => (Color::LightCyan, "[|]", "\u{2503}", "\u{f0db}"),
        Kanban => (Color::Magenta, "[K]", "\u{25a4}", "\u{f24d}"),
        Timeline => (Color::LightMagenta, "[T]", "\u{25c6}", "\u{f017}"),

        // ITS Theme - Data/info (neutral)
        Infobox => (Color::White, "[I]", "\u{24d8}", "\u{f05a}"),
        Metadata => (Color::DarkGray, "[M]", "\u{2630}", "\u{f0c9}"),
        Statblocks => (Color::Gray, "[S]", "\u{2637}", "\u{f080}"),

        // ITS Theme - Task/action (green family)
        Checks => (Color::Green, "[v]", "\u{2611}", "\u{f046}"),
        Kith => (Color::LightGreen, "[k]", "\u{2619}", "\u{f0c0}"),

        // ITS Theme - Neutral
        Blank => (Color::DarkGray, "[ ]", "\u{25cb}", "\u{f10c}"),
        Caption => (Color::DarkGray, "[c]", "\u{2014}", "\u{f036}"),
    }
}

fn callout_type_name(kind: &ast::BlockQuoteKind) -> &'static str {
    use ast::BlockQuoteKind::*;
    match kind {
        Note => "NOTE",
        Tip => "TIP",
        Important => "IMPORTANT",
        Warning => "WARNING",
        Caution => "CAUTION",
        Aside => "ASIDE",
        Blank => "BLANK",
        Caption => "CAPTION",
        Cards => "CARDS",
        Checks => "CHECKS",
        Column => "COLUMN",
        Grid => "GRID",
        Infobox => "INFOBOX",
        Kanban => "KANBAN",
        Kith => "KITH",
        Metadata => "METADATA",
        Quote => "QUOTE",
        Recite => "RECITE",
        Statblocks => "STATBLOCKS",
        Timeline => "TIMELINE",
    }
}

fn callout_icon(kind: &ast::BlockQuoteKind, preset: &crate::config::Preset) -> &'static str {
    let (_, ascii, unicode, nerdfont) = callout_style(kind);
    match preset {
        crate::config::Preset::NerdFont => nerdfont,
        crate::config::Preset::Ascii => ascii,
        _ => unicode,
    }
}

/// Returns `(color, ascii_icon, unicode_icon, nerdfont_icon)` for an ITS Theme task marker.
///
/// Standard variants (`Checked`, `Unchecked`) and `LooselyChecked` are handled directly
/// in `task()` and must not be passed here.
fn task_style(kind: &ast::TaskKind) -> (Color, &'static str, &'static str, &'static str) {
    use ast::TaskKind::*;
    // (color, ascii_icon, unicode_icon, nerdfont_icon)
    match kind {
        // Navigation / Time (cool tones — Cyan family)
        Forward => (Color::Cyan, "[>]", "\u{25b6}", "\u{f0da}"),     // nf-fa-caret_right
        Migrated => (Color::LightCyan, "[<]", "\u{25c0}", "\u{f0d9}"), // nf-fa-caret_left
        Date => (Color::Cyan, "[D]", "\u{1f4c5}", "\u{f073}"),        // nf-fa-calendar
        Time => (Color::LightCyan, "[T]", "\u{231a}", "\u{f017}"),    // nf-fa-clock_o
        Dropped => (Color::DarkGray, "[-]", "\u{2717}", "\u{f00d}"),  // nf-fa-times

        // Status / Completion (amber / muted tones)
        HalfDone => (Color::Yellow, "[/]", "\u{25d1}", "\u{f111}"),   // nf-fa-circle (half)
        Doing => (Color::LightYellow, "[d]", "\u{21bb}", "\u{f110}"), // nf-fa-spinner

        // Importance / Action (warm tones — Red/Orange family)
        Important => (Color::Red, "[!]", "\u{26a0}", "\u{f071}"),     // nf-fa-warning
        Add => (Color::LightRed, "[+]", "\u{ff0b}", "\u{f067}"),      // nf-fa-plus
        Pro => (Color::LightGreen, "[P]", "\u{2714}", "\u{f00c}"),    // nf-fa-check
        Con => (Color::Red, "[C]", "\u{2718}", "\u{f00d}"),           // nf-fa-times (distinct from Dropped by color)

        // Knowledge / Research (blue / purple family)
        Research => (Color::Blue, "[R]", "\u{2315}", "\u{f002}"),     // nf-fa-search
        Information => (Color::LightBlue, "[I]", "\u{2139}", "\u{f05a}"), // nf-fa-info_circle
        Idea => (Color::Magenta, "[i]", "\u{2605}", "\u{f0eb}"),      // nf-fa-lightbulb_o (star, distinct from Favorite)
        Brainstorm => (Color::LightMagenta, "[B]", "\u{26a1}", "\u{f0e7}"), // nf-fa-bolt

        // Writing / Reference (neutral / green family)
        Quote => (Color::Green, "[Q]", "\u{275d}", "\u{f10d}"),       // nf-fa-quote_left
        Note => (Color::LightGreen, "[N]", "\u{270e}", "\u{f249}"),   // nf-fa-sticky_note_o
        Talk => (Color::Green, "[t]", "\u{2709}", "\u{f075}"),        // nf-fa-comment (envelope as chat)
        Paraphrase => (Color::LightGreen, "[p]", "\u{21a9}", "\u{f064}"), // nf-fa-share

        // Creative / Story (magenta / dark family)
        World => (Color::Magenta, "[W]", "\u{2316}", "\u{f0ac}"),     // nf-fa-globe (crosshair)
        Outline => (Color::LightMagenta, "[O]", "\u{2261}", "\u{f0cb}"), // nf-fa-list_ol (triple bar)
        Foreshadow => (Color::Magenta, "[F]", "\u{2691}", "\u{f024}"), // nf-fa-flag (distinct from World by icon)
        Clue => (Color::LightMagenta, "[f]", "\u{2318}", "\u{f002}"), // nf-fa-search (command symbol, distinct)

        // Decision / Judgment (yellow / teal family)
        Question => (Color::Yellow, "[?]", "\u{2753}", "\u{f128}"),   // nf-fa-question
        Answer => (Color::LightYellow, "[A]", "\u{2713}", "\u{f00c}"), // nf-fa-check (distinct from Pro by color)
        Choice => (Color::Cyan, "[c]", "\u{25ce}", "\u{f192}"),       // nf-fa-dot_circle_o

        // Person / Place (earth tones)
        Character => (Color::Rgb(210, 140, 80), "[@]", "\u{25cf}", "\u{f007}"), // nf-fa-user
        Location => (Color::Rgb(180, 120, 60), "[L]", "\u{25b2}", "\u{f041}"),  // nf-fa-map_marker

        // Content / Meta (green family)
        Example => (Color::Green, "[E]", "\u{25b8}", "\u{f0a4}"),     // nf-fa-hand_o_right
        Bookmark => (Color::LightGreen, "[b]", "\u{25c6}", "\u{f02e}"), // nf-fa-bookmark
        Reward => (Color::LightGreen, "[r]", "\u{2605}", "\u{f091}"), // nf-fa-trophy (star, distinct from Idea by color)

        // Emotion / Symbol (bright / varied)
        Conflict => (Color::Red, "[~]", "\u{2717}", "\u{f0e7}"),      // nf-fa-bolt (distinct from Con by icon)
        Favorite => (Color::LightYellow, "[H]", "\u{2665}", "\u{f005}"), // nf-fa-star (heart, distinct)
        Symbolism => (Color::LightMagenta, "[&]", "\u{221e}", "\u{221e}"), // infinity symbol
        Secret => (Color::DarkGray, "[s]", "\u{1f512}", "\u{f023}"),  // nf-fa-lock

        // These variants are handled directly in task() — must not reach here
        Checked | Unchecked | LooselyChecked(_) => {
            unreachable!("Checked/Unchecked/LooselyChecked handled in task() directly")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Preset, Symbols};
    use crate::note_editor::rich_text::TextSegment;
    use insta::assert_snapshot;

    // Helper to convert VirtualBlock lines to snapshot-friendly string
    fn virtual_block_to_string(block: &VirtualBlock) -> String {
        block
            .lines
            .iter()
            .map(|line| {
                line.virtual_spans()
                    .iter()
                    .map(|span| match span {
                        VirtualSpan::Content(span, _) => span.content.to_string(),
                        VirtualSpan::Synthetic(span) => span.content.to_string(),
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // Test that callout_style returns the correct color for standard types
    #[test]
    fn test_callout_style_standard_colors() {
        let (note_color, _, _, _) = callout_style(&ast::BlockQuoteKind::Note);
        assert_eq!(note_color, Color::Cyan);

        let (tip_color, _, _, _) = callout_style(&ast::BlockQuoteKind::Tip);
        assert_eq!(tip_color, Color::Green);

        let (warning_color, _, _, _) = callout_style(&ast::BlockQuoteKind::Warning);
        assert_eq!(warning_color, Color::Yellow);

        let (caution_color, _, _, _) = callout_style(&ast::BlockQuoteKind::Caution);
        assert_eq!(caution_color, Color::Red);

        let (important_color, _, _, _) = callout_style(&ast::BlockQuoteKind::Important);
        assert_eq!(important_color, Color::Magenta);
    }

    // Test that callout_type_name returns uppercase strings
    #[test]
    fn test_callout_type_name_uppercase() {
        assert_eq!(callout_type_name(&ast::BlockQuoteKind::Note), "NOTE");
        assert_eq!(callout_type_name(&ast::BlockQuoteKind::Tip), "TIP");
        assert_eq!(
            callout_type_name(&ast::BlockQuoteKind::Important),
            "IMPORTANT"
        );
        assert_eq!(
            callout_type_name(&ast::BlockQuoteKind::Warning),
            "WARNING"
        );
        assert_eq!(callout_type_name(&ast::BlockQuoteKind::Caution), "CAUTION");
        assert_eq!(callout_type_name(&ast::BlockQuoteKind::Aside), "ASIDE");
        assert_eq!(callout_type_name(&ast::BlockQuoteKind::Kanban), "KANBAN");
        assert_eq!(
            callout_type_name(&ast::BlockQuoteKind::Statblocks),
            "STATBLOCKS"
        );
    }

    // Test that callout_icon returns different strings for different presets
    #[test]
    fn test_callout_icon_varies_by_preset() {
        let kind = ast::BlockQuoteKind::Note;
        let ascii_icon = callout_icon(&kind, &Preset::Ascii);
        let unicode_icon = callout_icon(&kind, &Preset::Unicode);
        let nerdfont_icon = callout_icon(&kind, &Preset::NerdFont);

        assert!(ascii_icon != unicode_icon);
        assert!(unicode_icon != nerdfont_icon);
        assert!(ascii_icon != nerdfont_icon);
    }

    // Insta snapshot tests for block_quote() render output
    #[test]
    fn snapshot_block_quote_note_callout() {
        let kind = Some(ast::BlockQuoteKind::Note);
        let title = None;
        let symbols = Symbols::unicode();
        let prefix = Span::raw("");
        let max_width = 80;
        let content = "Test body content.".to_string();
        let source_range = 0..20;

        let block = block_quote(
            &content,
            prefix,
            &kind,
            &title,
            &[ast::Node::Paragraph {
                text: RichText::from([TextSegment::plain("Test body content.")]),
                source_range: source_range.clone(),
            }],
            &source_range,
            max_width,
            &RenderStyle::Visual,
            &symbols,
        );

        insta::assert_snapshot!(virtual_block_to_string(&block));
    }

    #[test]
    fn snapshot_block_quote_warning_callout() {
        let kind = Some(ast::BlockQuoteKind::Warning);
        let title = None;
        let symbols = Symbols::unicode();
        let prefix = Span::raw("");
        let max_width = 80;
        let content = "Warning message here.".to_string();
        let source_range = 0..21;

        let block = block_quote(
            &content,
            prefix,
            &kind,
            &title,
            &[ast::Node::Paragraph {
                text: RichText::from([TextSegment::plain("Warning message here.")]),
                source_range: source_range.clone(),
            }],
            &source_range,
            max_width,
            &RenderStyle::Visual,
            &symbols,
        );

        insta::assert_snapshot!(virtual_block_to_string(&block));
    }

    #[test]
    fn snapshot_block_quote_aside_callout() {
        let kind = Some(ast::BlockQuoteKind::Aside);
        let title = None;
        let symbols = Symbols::unicode();
        let prefix = Span::raw("");
        let max_width = 80;
        let content = "This is an aside.".to_string();
        let source_range = 0..17;

        let block = block_quote(
            &content,
            prefix,
            &kind,
            &title,
            &[ast::Node::Paragraph {
                text: RichText::from([TextSegment::plain("This is an aside.")]),
                source_range: source_range.clone(),
            }],
            &source_range,
            max_width,
            &RenderStyle::Visual,
            &symbols,
        );

        insta::assert_snapshot!(virtual_block_to_string(&block));
    }

    #[test]
    fn snapshot_block_quote_note_with_title() {
        let kind = Some(ast::BlockQuoteKind::Note);
        let title = Some("My Title".to_string());
        let symbols = Symbols::unicode();
        let prefix = Span::raw("");
        let max_width = 80;
        let content = "Test body content.".to_string();
        let source_range = 0..20;

        let block = block_quote(
            &content,
            prefix,
            &kind,
            &title,
            &[ast::Node::Paragraph {
                text: RichText::from([TextSegment::plain("Test body content.")]),
                source_range: source_range.clone(),
            }],
            &source_range,
            max_width,
            &RenderStyle::Visual,
            &symbols,
        );

        insta::assert_snapshot!(virtual_block_to_string(&block));
    }

    #[test]
    fn snapshot_block_quote_plain() {
        let kind = None;
        let title = None;
        let symbols = Symbols::unicode();
        let prefix = Span::raw("");
        let max_width = 80;
        let content = "Plain blockquote text.".to_string();
        let source_range = 0..21;

        let block = block_quote(
            &content,
            prefix,
            &kind,
            &title,
            &[ast::Node::Paragraph {
                text: RichText::from([TextSegment::plain("Plain blockquote text.")]),
                source_range: source_range.clone(),
            }],
            &source_range,
            max_width,
            &RenderStyle::Visual,
            &symbols,
        );

        insta::assert_snapshot!(virtual_block_to_string(&block));
    }

    // Test that all 35 ITS Theme TaskKind variants have unique icon+color combinations
    #[test]
    fn test_task_style_no_duplicates() {
        use ast::TaskKind::*;
        let all_kinds = [
            Dropped, Forward, Migrated, Date, Question, HalfDone, Add, Research, Important, Idea,
            Brainstorm, Pro, Con, Quote, Note, Bookmark, Information, Paraphrase, Location,
            Example, Answer, Reward, Choice, Doing, Time, Character, Talk, Outline, Conflict,
            World, Clue, Foreshadow, Favorite, Symbolism, Secret,
        ];
        let styles: Vec<_> = all_kinds.iter().map(task_style).collect();
        // All NerdFont icon+color pairs should be unique (no two markers look identical)
        let pairs: std::collections::HashSet<_> = styles
            .iter()
            .map(|(color, _, _, nf)| (format!("{:?}", color), *nf))
            .collect();
        assert_eq!(
            pairs.len(),
            all_kinds.len(),
            "duplicate icon+color combination detected"
        );
    }

    // Test that task_style returns distinct colors for visually important pairs
    #[test]
    fn test_task_style_key_variants() {
        let (forward_color, _, _, _) = task_style(&ast::TaskKind::Forward);
        assert_eq!(forward_color, Color::Cyan);

        let (important_color, _, _, _) = task_style(&ast::TaskKind::Important);
        assert_eq!(important_color, Color::Red);

        let (con_color, _, _, _) = task_style(&ast::TaskKind::Con);
        let (choice_color, _, _, _) = task_style(&ast::TaskKind::Choice);
        // Con (red) and Choice (cyan) must be distinct
        assert_ne!(
            format!("{:?}", con_color),
            format!("{:?}", choice_color),
            "Con and Choice should have different colors"
        );
    }

    // --- Table rendering snapshot tests ---

    fn render_table_to_string(md: &str, symbols: &Symbols) -> String {
        use crate::note_editor::parser;
        let nodes = parser::from_str(md);
        let table_node = nodes
            .iter()
            .find(|n| matches!(n, ast::Node::Table { .. }))
            .expect("no table node found in parsed markdown");
        if let ast::Node::Table {
            alignments,
            header,
            rows,
            source_range,
        } = table_node
        {
            let block = table(
                alignments,
                header,
                rows,
                source_range,
                80,
                &RenderStyle::Visual,
                symbols,
                0, // h_scroll = 0 for standard rendering
            );
            virtual_block_to_string(&block)
        } else {
            panic!("expected Table node");
        }
    }

    #[test]
    fn table_ascii_preset() {
        let md = "| Name  | Age | City   |\n| :---- | --: | :----: |\n| Alice |  28 | London |\n| Bob   |  34 | Paris  |\n";
        let s = render_table_to_string(md, &Symbols::ascii());
        insta::assert_snapshot!("table_ascii", s);
    }

    #[test]
    fn table_unicode_preset() {
        let md = "| Name  | Age | City   |\n| :---- | --: | :----: |\n| Alice |  28 | London |\n| Bob   |  34 | Paris  |\n";
        let s = render_table_to_string(md, &Symbols::unicode());
        insta::assert_snapshot!("table_unicode", s);
    }

    #[test]
    fn table_unicode_wide_chars() {
        // CJK chars are width-2, so columns must be wider
        let md = "| Name | Score |\n| :--- | ----: |\n| 日本  |   100 |\n| 中国  |    99 |\n";
        let s = render_table_to_string(md, &Symbols::unicode());
        insta::assert_snapshot!("table_unicode_wide_chars", s);
    }

    /// Verify that ◀ and ▶ scroll indicators appear when h_scroll > 0 and the table
    /// extends beyond max_width (D-14, D-15).
    #[test]
    fn table_h_scroll_shows_indicators() {
        use crate::note_editor::parser;

        let md =
            "| A | B | C | D | E |\n| - | - | - | - | - |\n| 1 | 2 | 3 | 4 | 5 |\n";
        let nodes = parser::from_str(md);
        let table_node = nodes
            .iter()
            .find(|n| matches!(n, ast::Node::Table { .. }))
            .expect("no Table node found");

        if let ast::Node::Table {
            alignments,
            header,
            rows,
            source_range,
        } = table_node
        {
            // h_scroll=4, max_width=12 — content is hidden on both sides, forcing both indicators
            let block = table(
                alignments,
                header,
                rows,
                source_range,
                12,
                &RenderStyle::Visual,
                &Symbols::unicode(),
                4,
            );
            // Collect all span text in the first line (top border)
            // Use spans() which returns Vec<Span<'a>> after consuming the VirtualLine
            let first_line: String = block.lines[0]
                .clone()
                .spans()
                .iter()
                .map(|s| s.content.as_ref().to_string())
                .collect();
            assert!(
                first_line.contains('◀'),
                "expected left scroll indicator ◀ in top border, got: {first_line:?}"
            );
            assert!(
                first_line.contains('▶'),
                "expected right scroll indicator ▶ in top border, got: {first_line:?}"
            );
            // Also verify that h_scroll=0 does NOT show left indicator
            let block_noscroll = table(
                alignments,
                header,
                rows,
                source_range,
                80,
                &RenderStyle::Visual,
                &Symbols::unicode(),
                0,
            );
            let first_line_noscroll: String = block_noscroll.lines[0]
                .clone()
                .spans()
                .iter()
                .map(|s| s.content.as_ref().to_string())
                .collect();
            assert!(
                !first_line_noscroll.contains('◀'),
                "should not have left indicator when h_scroll=0, got: {first_line_noscroll:?}"
            );
        } else {
            panic!("expected Table node");
        }
    }

    // === Phase 7 Footnote tests (Wave 2 — 07-02 implementation) ===

    #[test]
    fn test_render_footnote_ref() {
        let rt = RichText::from(vec![InlineNode::FootnoteRef("1".to_string())]);
        let spans = rich_text_to_spans(&rt);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "[1]");
        // Verify color is LightCyan
        assert_eq!(spans[0].style.fg, Some(Color::LightCyan));
    }

    #[test]
    fn snapshot_footnote_section() {
        let symbols = Symbols::unicode();
        let mut defs = IndexMap::new();
        defs.insert(
            "1".to_string(),
            RichText::from(vec![TextSegment::plain("First footnote content")]),
        );
        defs.insert(
            "2".to_string(),
            RichText::from(vec![TextSegment::plain("Second footnote content")]),
        );

        let block = render_footnote_section(&defs, &(100..200), 40, &symbols);
        let lines: Vec<String> = block.lines.iter().map(|line| {
            line.virtual_spans().iter().map(|span| {
                match span {
                    VirtualSpan::Synthetic(s) | VirtualSpan::Content(s, _) => s.content.to_string(),
                }
            }).collect::<String>()
        }).collect();

        assert_snapshot!(lines.join("\n"));
    }

    // === Phase 7 External link tests (Wave 3 — 07-03 implementation) ===

    #[test]
    fn test_render_link_style() {
        use crate::note_editor::rich_text::{InlineNode, LinkTarget};

        let rt = RichText::from(vec![InlineNode::Link {
            text: "click here".to_string(),
            target: LinkTarget::External("https://example.com".to_string()),
        }]);
        let spans = rich_text_to_spans(&rt);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "click here");
        assert_eq!(spans[0].style.fg, Some(Color::LightCyan));
        assert!(spans[0].style.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn test_non_http_link_no_osc8() {
        // Verify that non-http links parsed as plain text do not produce Link nodes
        use crate::note_editor::{parser, rich_text::InlineNode};

        let nodes = parser::from_str("Mail [me](mailto:a@b.com)");
        let paragraph = &nodes[0];
        if let crate::note_editor::ast::Node::Paragraph { text, .. } = paragraph {
            for node in text.nodes() {
                // None of the nodes should be InlineNode::Link
                assert!(
                    !matches!(node, InlineNode::Link { .. }),
                    "mailto link should not produce InlineNode::Link"
                );
            }
        } else {
            panic!("Expected Paragraph node");
        }
    }
}
