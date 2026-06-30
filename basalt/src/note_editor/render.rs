use ratatui::{
    style::{Color, Modifier, Style, Stylize},
    text::Span,
};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    config::Symbols,
    note_editor::{
        ast::{self, SourceRange},
        rich_text::RichText,
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

/// Background for code blocks: the terminal theme's black, so it follows the
/// user's colour scheme rather than a hard-coded shade.
const CODE_BG: Color = Color::Black;

#[derive(Clone, PartialEq, Debug)]
pub enum RenderStyle {
    Raw,
    Visual,
    Reader,
}

impl RenderStyle {
    pub fn is_styled(&self) -> bool {
        matches!(self, Self::Visual | Self::Reader)
    }
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
                (Some(marker), true) if option.is_styled() => virtual_line!([
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
        // Tabs are kept here so the content span stays byte-aligned with the
        // source (the cursor maps by byte offset); they are expanded to spaces
        // at draw time so they don't break the terminal layout.
        line,
        Style::default(),
        prefix,
        source_range,
        max_width,
        None,
        &RenderStyle::Raw,
        symbols,
    )
}

/// Display width of source text, counting tabs as two columns to match how they
/// are expanded at draw time and how the cursor measures columns.
fn display_width(text: &str) -> usize {
    text.chars()
        .map(|c| if c == '\t' { 2 } else { c.width().unwrap_or(0) })
        .sum()
}

/// Renders source lines for edit mode, keeping a 1:1 mapping between source and
/// display lines. The line under the cursor is shown raw (so its markers and
/// indentation are editable); every other line is decorated in place — the
/// leading marker is replaced with its rendered icon while the text after it
/// stays byte-aligned with the source. This mirrors how editing-markdown
/// previews work (decorate, reveal raw on the cursor line) and keeps nested
/// lists and structural edits reliable.
pub fn edit_lines<'a>(
    content: &str,
    base: usize,
    cursor_offset: usize,
    max_width: usize,
    horizontal_offset: usize,
    symbols: &Symbols,
) -> Vec<VirtualLine<'a>> {
    // Full-width fills (code backgrounds, heading rules) extend by the horizontal
    // scroll so they still span the viewport once it pans to follow the cursor.
    let fill_width = max_width + horizontal_offset;

    let mut lines = Vec::new();
    let mut start = base;
    // Lines inside a fenced code block are literal — never decorated as markdown.
    let mut in_code = false;

    for line in content.split_inclusive('\n') {
        let line_range = start..start + line.len();
        start = line_range.end;
        let text = line.strip_suffix('\n').unwrap_or(line);
        let fence = is_code_fence(text);

        if in_code || fence {
            lines.push(code_line(text, &line_range, fill_width));
            // A fence toggles the block: the opener enters it, the next closes it.
            in_code ^= fence;
        } else {
            lines.extend(edit_line(
                text,
                &line_range,
                line_range.contains(&cursor_offset),
                max_width,
                fill_width,
                symbols,
            ));
        }
    }
    lines
}

/// Renders a single non-code source line for edit mode.
fn edit_line<'a>(
    text: &str,
    line_range: &SourceRange<usize>,
    is_cursor: bool,
    max_width: usize,
    fill_width: usize,
    symbols: &Symbols,
) -> Vec<VirtualLine<'a>> {
    if text.is_empty() {
        return vec![virtual_line!([content_span!(
            "".to_string(),
            line_range.clone()
        )])];
    }

    let indent_len = text.len() - text.trim_start().len();
    let rest = &text[indent_len..];

    // Headings always keep their rendered style (bold, colour, underline) while
    // editing; the `#` markers stay visible but dimmed so they can still be edited.
    if let Some(level) = heading_level(rest) {
        return heading_lines(text, line_range, indent_len, level, fill_width, symbols);
    }

    if is_cursor {
        // Raw, but keep the block-quote markers coloured.
        if let Some((prefix_len, _)) = quote_prefix(rest) {
            return vec![raw_quote_line(text, line_range, indent_len, prefix_len)];
        }
        return render_raw_line(text, Span::default(), line_range, max_width, symbols);
    }

    decorate_line(text, line_range, max_width, symbols)
}

/// True if the line opens or closes a fenced code block (``` ``` ``` or `~~~`).
fn is_code_fence(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

/// Renders a code-block line literally, with the code background, so its content
/// is never interpreted as markdown.
fn code_line<'a>(
    text: &str,
    line_range: &SourceRange<usize>,
    fill_width: usize,
) -> VirtualLine<'a> {
    let code_bg = Style::new().bg(CODE_BG);
    let pad = fill_width.saturating_sub(display_width(text) + 1);
    virtual_line!([
        synthetic_span!(Span::styled(" ", code_bg)),
        content_span!(Span::raw(text.to_string()).bg(CODE_BG), line_range.clone()),
        synthetic_span!(Span::styled(" ".repeat(pad), code_bg))
    ])
}

/// Renders a heading line for edit mode: the `#` markers are kept (dimmed) so
/// they remain editable, the title carries its heading style, and H1/H2 get
/// their underline rule — matching how headings look when not editing.
fn heading_lines<'a>(
    text: &str,
    line_range: &SourceRange<usize>,
    indent_len: usize,
    level: usize,
    fill_width: usize,
    symbols: &Symbols,
) -> Vec<VirtualLine<'a>> {
    let marker_end = (indent_len + level + 1).min(text.len());
    let title = &text[marker_end..];
    let start = line_range.start;

    let mut lines = vec![virtual_line!([
        content_span!(text[..indent_len].to_string(), start..start + indent_len),
        content_span!(
            text[indent_len..marker_end].to_string().dark_gray(),
            (start + indent_len)..(start + marker_end)
        ),
        content_span!(
            heading_span(title, level),
            (start + marker_end)..line_range.end
        )
    ])];

    match level {
        1 => lines.push(virtual_line!([synthetic_span!(Span::raw(
            symbols.h1_underline.repeat(fill_width)
        ))])),
        2 => lines.push(virtual_line!([synthetic_span!(symbols
            .h2_underline
            .repeat(fill_width)
            .yellow())])),
        _ => {}
    }
    lines
}

/// Renders the cursor's block-quote line raw, but with the `>` markers (all
/// nesting levels) coloured to match the rendered `┃` markers on other lines.
fn raw_quote_line<'a>(
    text: &str,
    line_range: &SourceRange<usize>,
    indent_len: usize,
    prefix_len: usize,
) -> VirtualLine<'a> {
    let start = line_range.start;
    let marker_end = indent_len + prefix_len;
    virtual_line!([
        content_span!(text[..indent_len].to_string(), start..start + indent_len),
        content_span!(
            text[indent_len..marker_end].to_string().magenta(),
            (start + indent_len)..(start + marker_end)
        ),
        content_span!(
            text[marker_end..].to_string(),
            (start + marker_end)..line_range.end
        )
    ])
}

/// Renders one non-cursor source line with its marker replaced by a rendered
/// icon. The content span starts after the marker, so it stays byte-aligned
/// with the source and the cursor can target it (revealing the raw line).
fn decorate_line<'a>(
    text: &str,
    line_range: &SourceRange<usize>,
    max_width: usize,
    symbols: &Symbols,
) -> Vec<VirtualLine<'a>> {
    let indent_len = text.len() - text.trim_start().len();
    let rest = &text[indent_len..];
    // Expand tabs to spaces (the terminal collapses raw tabs, which would hide
    // the indentation) and derive nesting depth from the visible width.
    let indent = text[..indent_len].replace('\t', "  ");
    let depth = indent.chars().count() / 2;
    let prefix = Span::raw(indent);

    let render = |marker: Option<Span<'static>>, content_start: usize, content: Span<'a>| {
        let content_range = (line_range.start + content_start)..line_range.end;
        text_wrap(
            &content,
            prefix.clone(),
            &content_range,
            max_width,
            marker,
            &RenderStyle::Visual,
            symbols,
        )
    };

    if let Some((checked, marker_len)) = task_marker(rest) {
        let content_start = indent_len + marker_len;
        let (icon, content) = if checked {
            (
                format!("{} ", symbols.task_checked).magenta(),
                Span::raw(text[content_start..].to_string())
                    .dark_gray()
                    .add_modifier(Modifier::CROSSED_OUT),
            )
        } else {
            (
                format!("{} ", symbols.task_unchecked).dark_gray(),
                Span::raw(text[content_start..].to_string()),
            )
        };
        return render(Some(icon), content_start, content);
    }
    if let Some(marker_len) = unordered_marker(rest) {
        let bullet = if symbols.list_markers.is_empty() {
            "-"
        } else {
            &symbols.list_markers[depth % symbols.list_markers.len()]
        };
        let content_start = indent_len + marker_len;
        return render(
            Some(format!("{bullet} ").dark_gray()),
            content_start,
            Span::raw(text[content_start..].to_string()),
        );
    }
    if let Some(marker_len) = ordered_marker(rest) {
        let content_start = indent_len + marker_len;
        return render(
            Some(rest[..marker_len].to_string().dark_gray()),
            content_start,
            Span::raw(text[content_start..].to_string()),
        );
    }
    if let Some((prefix_len, levels)) = quote_prefix(rest) {
        let content_start = indent_len + prefix_len;
        return render(
            Some(Span::raw("┃ ".repeat(levels)).magenta()),
            content_start,
            Span::raw(text[content_start..].to_string()),
        );
    }

    render(None, indent_len, Span::raw(rest.to_string()))
}

/// Number of leading `#` for an ATX heading (`# ` .. `###### `), else `None`.
fn heading_level(rest: &str) -> Option<usize> {
    let hashes = rest.chars().take_while(|c| *c == '#').count();
    ((1..=6).contains(&hashes) && rest.as_bytes().get(hashes) == Some(&b' ')).then_some(hashes)
}

/// Byte length of a `- [ ] ` task marker and whether it is checked.
fn task_marker(rest: &str) -> Option<(bool, usize)> {
    let bytes = rest.as_bytes();
    let valid = matches!(bytes.first(), Some(b'-' | b'*' | b'+'))
        && bytes.get(1) == Some(&b' ')
        && bytes.get(2) == Some(&b'[')
        && bytes.get(4) == Some(&b']')
        && bytes.get(5) == Some(&b' ');
    valid.then(|| (matches!(bytes.get(3), Some(b'x' | b'X')), 6))
}

/// Byte length of a `- `/`* `/`+ ` unordered list marker.
fn unordered_marker(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    (matches!(bytes.first(), Some(b'-' | b'*' | b'+')) && bytes.get(1) == Some(&b' ')).then_some(2)
}

/// Byte length of a `1. `/`1) ` ordered list marker.
fn ordered_marker(rest: &str) -> Option<usize> {
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    let bytes = rest.as_bytes();
    (digits > 0
        && matches!(bytes.get(digits), Some(b'.' | b')'))
        && bytes.get(digits + 1) == Some(&b' '))
    .then_some(digits + 2)
}

/// Byte length and nesting level of a block-quote prefix, consuming every
/// leading `>` marker (e.g. `> > ` is two levels) so nested quotes are coloured
/// in full.
fn quote_prefix(rest: &str) -> Option<(usize, usize)> {
    let bytes = rest.as_bytes();
    let mut len = 0;
    let mut levels = 0;
    while bytes.get(len) == Some(&b'>') {
        len += 1;
        levels += 1;
        if bytes.get(len) == Some(&b' ') {
            len += 1;
        }
    }
    (levels > 0).then_some((len, levels))
}

fn heading_span<'a>(text: &str, level: usize) -> Span<'a> {
    let span = Span::raw(text.to_string()).bold();
    match level {
        2 => span.yellow(),
        3 => span.cyan(),
        4 => span.magenta(),
        _ => span,
    }
}

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
    horizontal_offset: usize,
    option: &RenderStyle,
    symbols: &Symbols,
) -> VirtualBlock<'a> {
    use ast::HeadingLevel::*;

    let (text, heading_source_range, remaining) = match option {
        RenderStyle::Visual | RenderStyle::Reader => (text.to_string(), source_range.clone(), None),
        RenderStyle::Raw => {
            let node_content = content.get(source_range.clone()).unwrap_or(content);
            match node_content.split_once('\n') {
                Some((first, rest)) => {
                    let end = (source_range.start + first.len()).min(source_range.end);
                    (
                        first.to_string(),
                        source_range.start..end,
                        Some(!rest.is_empty()),
                    )
                }
                None => (node_content.to_string(), source_range.clone(), None),
            }
        }
    };

    let prefix_width = prefix.width();

    let h = |marker: Span<'static>, content: Span<'a>| {
        let mut wrapped_heading = text_wrap(
            &content,
            prefix.clone(),
            &heading_source_range,
            max_width,
            Some(marker),
            option,
            symbols,
        );

        if option.is_styled() {
            wrapped_heading.push(empty_virtual_line!());
        }
        wrapped_heading
    };

    let h_with_underline = |content: Span<'a>, underline: Span<'static>| {
        let mut wrapped_heading = text_wrap(
            &content,
            prefix.clone(),
            &heading_source_range,
            max_width,
            None,
            option,
            symbols,
        );
        wrapped_heading.push(virtual_line!([synthetic_span!(underline)]));
        wrapped_heading
    };

    // Extend the underline by the horizontal scroll so it still spans the viewport when panned.
    let underline_width = (max_width + horizontal_offset).saturating_sub(prefix_width);
    let mut lines = match level {
        H1 => h_with_underline(
            if option.is_styled() {
                text.to_uppercase().bold()
            } else {
                text.bold()
            },
            symbols.h1_underline.repeat(underline_width).into(),
        ),
        H2 => h_with_underline(
            text.bold().yellow(),
            symbols.h2_underline.repeat(underline_width).yellow(),
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

    if remaining == Some(true) {
        // +1 skip the `\n` that we split on.
        let remaining_start = (heading_source_range.end + 1).min(source_range.end);
        let remaining_range = remaining_start..source_range.end;
        lines.extend(render_raw(
            content,
            &remaining_range,
            max_width,
            prefix.clone(),
            symbols,
        ));
    }

    VirtualBlock::new(&lines, source_range)
}

pub fn render_raw<'a>(
    content: &str,
    source_range: &SourceRange<usize>,
    max_width: usize,
    prefix: Span<'static>,
    symbols: &Symbols,
) -> Vec<VirtualLine<'a>> {
    // `content` is the whole document; slice down to this node's source text.
    let content = content.get(source_range.clone()).unwrap_or(content);
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
            synthetic_span!(prefix.clone()),
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
        RenderStyle::Visual | RenderStyle::Reader => {
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

// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
pub fn code_block<'a>(
    content: &str,
    prefix: Span<'static>,
    // TODO: Add lang support
    // Ref: https://github.com/erikjuhani/basalt/issues/96
    _lang: &Option<String>,
    text: &RichText,
    source_range: &SourceRange<usize>,
    max_width: usize,
    horizontal_offset: usize,
    option: &RenderStyle,
) -> VirtualBlock<'a> {
    // Extend the background by the horizontal scroll so it still spans the viewport when panned.
    let fill_width = max_width + horizontal_offset;
    let lines = match option {
        RenderStyle::Raw => {
            let content = content.get(source_range.clone()).unwrap_or(content);
            let mut current_range_start = source_range.start;

            let mut lines = content
                .lines()
                .map(|line| {
                    let line_range = line_range(current_range_start, line.len(), true);
                    current_range_start = line_range.end;

                    virtual_line!([
                        synthetic_span!(prefix.clone()),
                        synthetic_span!(Span::styled(" ", Style::new().bg(CODE_BG))),
                        content_span!(line.to_string().bg(CODE_BG), line_range),
                        synthetic_span!(" "
                            .repeat(
                                fill_width
                                    .saturating_sub(prefix.width() + line.chars().count())
                                    .saturating_sub(1)
                            )
                            .bg(CODE_BG)),
                    ])
                })
                .collect::<Vec<_>>();

            lines.push(empty_virtual_line!());
            lines
        }
        RenderStyle::Visual | RenderStyle::Reader => {
            let text = text.to_string();

            let padding_line = virtual_line!([
                synthetic_span!(prefix.clone()),
                synthetic_span!(" "
                    .repeat(fill_width.saturating_sub(prefix.width()))
                    .bg(CODE_BG))
            ]);

            let mut current_range_start = source_range.start;

            let mut lines = vec![padding_line.clone()];
            lines.extend(text.lines().map(|line| {
                let source_range = line_range(current_range_start, line.len(), true);
                current_range_start = source_range.end;

                virtual_line!([
                    synthetic_span!(prefix.clone()),
                    synthetic_span!(Span::styled(" ", Style::new().bg(CODE_BG))),
                    content_span!(line.to_string().bg(CODE_BG), source_range),
                    synthetic_span!(" "
                        .repeat(
                            fill_width
                                .saturating_sub(prefix.width() + line.chars().count())
                                .saturating_sub(1)
                        )
                        .bg(CODE_BG)),
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
    horizontal_offset: usize,
    option: &RenderStyle,
    symbols: &Symbols,
    list_depth: usize,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
        RenderStyle::Visual | RenderStyle::Reader => {
            let preserve_empty_lines = matches!(option, RenderStyle::Visual);
            let mut lines: Vec<VirtualLine<'a>> = nodes
                .iter()
                .enumerate()
                .flat_map(|(i, node)| {
                    let mut lines = Vec::new();

                    if preserve_empty_lines && i > 0 {
                        let prev_slice = content
                            .get(nodes[i - 1].source_range().clone())
                            .unwrap_or("");
                        let empties = trailing_empty_lines(prev_slice);
                        lines.extend(
                            (0..empties).map(|_| virtual_line!([synthetic_span!(prefix.clone())])),
                        );
                    }

                    lines.extend(
                        render_node(
                            content.to_string(),
                            node,
                            max_width,
                            horizontal_offset,
                            prefix.clone(),
                            option,
                            symbols,
                            list_depth,
                        )
                        .lines,
                    );
                    lines
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

/// Counts trailing empty lines at the end of a source slice.
pub(crate) fn trailing_empty_lines(slice: &str) -> usize {
    slice
        .lines()
        .rev()
        .take_while(|line| line.trim().is_empty())
        .count()
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
    horizontal_offset: usize,
    option: &RenderStyle,
    symbols: &Symbols,
    list_depth: usize,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
        RenderStyle::Visual | RenderStyle::Reader => {
            let Some((text, rest)) = nodes.split_first().and_then(|(first, rest)| {
                let text = first.rich_text()?;
                Some((text, rest))
            }) else {
                return VirtualBlock::new(&[], source_range);
            };

            let text = text.to_string();
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
                    horizontal_offset,
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
    horizontal_offset: usize,
    option: &RenderStyle,
    symbols: &Symbols,
    list_depth: usize,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
        RenderStyle::Visual | RenderStyle::Reader => {
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
                    horizontal_offset,
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
    horizontal_offset: usize,
    option: &RenderStyle,
    symbols: &Symbols,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
        RenderStyle::Visual | RenderStyle::Reader => nodes
            .iter()
            .enumerate()
            .flat_map(|(i, node)| {
                let mut lines = render_node(
                    content.to_string(),
                    node,
                    max_width,
                    horizontal_offset,
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

/// Every table column is at least one cell wide so a border is always drawable.
const MIN_COLUMN_WIDTH: usize = 1;

// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
pub fn table<'a>(
    content: &str,
    prefix: Span<'static>,
    alignments: &[ast::Alignment],
    head: &[RichText],
    rows: &[Vec<RichText>],
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
    symbols: &Symbols,
) -> VirtualBlock<'a> {
    let lines = match option {
        RenderStyle::Raw => render_raw(content, source_range, max_width, prefix, symbols),
        // The table renders boxed both for reading and while editing (when this is
        // not the active block). The active block reveals the cursor's row raw via
        // `edit_table`.
        RenderStyle::Visual | RenderStyle::Reader => render_table(
            content,
            &prefix,
            alignments,
            head,
            rows,
            source_range,
            max_width,
        ),
    };

    VirtualBlock::new(&lines, source_range)
}

/// Renders the active table block while editing: every row is boxed except the one under the
/// cursor, which is shown raw so its pipes stay byte-aligned with the source and stay editable —
/// the same "decorate, reveal raw on the cursor line" pattern used for lists and headings.
///
/// Columns and alignments are derived from the live buffer (not the possibly stale AST) so the box
/// tracks in-flight edits. Until the buffer holds a header and a delimiter row, the whole block is
/// edited raw line by line.
pub fn edit_table<'a>(
    content: &str,
    base: usize,
    cursor_offset: usize,
    max_width: usize,
    horizontal_offset: usize,
    symbols: &Symbols,
) -> Vec<VirtualLine<'a>> {
    let prefix = Span::default();

    let mut source_lines = Vec::new();
    let mut start = base;
    for line in content.split_inclusive('\n') {
        let range = start..start + line.len();
        start = range.end;
        source_lines.push((line.strip_suffix('\n').unwrap_or(line).to_string(), range));
    }
    while source_lines
        .last()
        .is_some_and(|(text, _)| text.trim().is_empty())
    {
        source_lines.pop();
    }

    // Box only a buffer that actually parses as a table — the authoritative test,
    // matching exactly what read mode shows. The moment the syntax breaks (a bad
    // delimiter, a mismatched column count, a deleted row), it parses as a paragraph
    // instead, so we fall back to raw line-by-line editing and the broken markdown
    // stays visible and fixable.
    if !matches!(
        crate::note_editor::parser::from_str(content).as_slice(),
        [ast::Node::Table { .. }]
    ) {
        return edit_lines(
            content,
            base,
            cursor_offset,
            max_width,
            horizontal_offset,
            symbols,
        );
    }

    let mut header = split_table_row(&source_lines[0].0);
    let alignments = split_table_row(&source_lines[1].0)
        .iter()
        .map(|cell| parse_alignment(cell))
        .collect::<Vec<_>>();

    let mut body = source_lines[2..]
        .iter()
        .map(|(text, _)| split_table_row(text))
        .collect::<Vec<_>>();

    let columns = header
        .len()
        .max(alignments.len())
        .max(body.iter().map(Vec::len).max().unwrap_or(0));
    header.resize(columns, String::new());
    body.iter_mut()
        .for_each(|row| row.resize(columns, String::new()));

    let available = max_width.saturating_sub(prefix.width());
    let widths = table_column_widths(&header, &body, columns, available);

    let on_cursor = |range: &SourceRange<usize>| range.contains(&cursor_offset);
    let raw = |text: &str, range: &SourceRange<usize>| {
        render_raw_line(text, prefix.clone(), range, max_width, symbols)
    };

    let mut lines = vec![table_rule(&prefix, &widths, None, '┌', '┬', '┐')];

    let (header_text, header_range) = &source_lines[0];
    if on_cursor(header_range) {
        lines.extend(raw(header_text, header_range));
    } else {
        lines.extend(table_content_rows(
            &prefix,
            &header,
            &widths,
            &alignments,
            true,
            Some(header_range.clone()),
        ));
    }

    // The delimiter row renders as a separator unless the cursor is on it. Anchoring its source
    // range keeps it reachable so it can be edited. With no body rows it closes the box.
    let (delimiter_text, delimiter_range) = &source_lines[1];
    if on_cursor(delimiter_range) {
        lines.extend(raw(delimiter_text, delimiter_range));
        if body.is_empty() {
            lines.push(table_rule(&prefix, &widths, None, '└', '┴', '┘'));
        }
    } else {
        let (left, junction, right) = if body.is_empty() {
            ('└', '┴', '┘')
        } else {
            ('├', '┼', '┤')
        };
        lines.push(table_rule(
            &prefix,
            &widths,
            Some(delimiter_range.clone()),
            left,
            junction,
            right,
        ));
    }

    for (i, row) in body.iter().enumerate() {
        let (text, range) = &source_lines[i + 2];
        if on_cursor(range) {
            lines.extend(raw(text, range));
        } else {
            lines.extend(table_content_rows(
                &prefix,
                row,
                &widths,
                &alignments,
                false,
                Some(range.clone()),
            ));
        }
        let (left, junction, right) = if i + 1 == body.len() {
            ('└', '┴', '┘')
        } else {
            ('├', '┼', '┤')
        };
        lines.push(table_rule(&prefix, &widths, None, left, junction, right));
    }

    lines
}

/// Splits a table source row on its pipes into trimmed cell strings, tolerating rows with or
/// without the optional leading and trailing pipe.
fn split_table_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('|').unwrap_or(trimmed);
    trimmed
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

/// Reads a column's alignment from its delimiter cell (`:---`, `:--:`, `---:`).
fn parse_alignment(cell: &str) -> ast::Alignment {
    let cell = cell.trim();
    match (cell.starts_with(':'), cell.ends_with(':')) {
        (true, true) => ast::Alignment::Center,
        (false, true) => ast::Alignment::Right,
        (true, false) => ast::Alignment::Left,
        (false, false) => ast::Alignment::None,
    }
}

/// Renders a bordered table whose columns shrink to fit `max_width`, wrapping long cell text onto
/// extra rows. Border and padding cells are synthetic; the first display line of each source row
/// carries that row's source range so the read-mode cursor can traverse the table.
fn render_table<'a>(
    content: &str,
    prefix: &Span<'static>,
    alignments: &[ast::Alignment],
    head: &[RichText],
    rows: &[Vec<RichText>],
    source_range: &SourceRange<usize>,
    max_width: usize,
) -> Vec<VirtualLine<'a>> {
    let columns = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
    if columns == 0 {
        return vec![empty_virtual_line!()];
    }

    let header = table_cells(head, columns);
    let body = rows
        .iter()
        .map(|row| table_cells(row, columns))
        .collect::<Vec<_>>();

    let available = max_width.saturating_sub(prefix.width());
    let widths = table_column_widths(&header, &body, columns, available);
    let source_lines = table_source_lines(content, source_range);

    let mut lines = vec![table_rule(prefix, &widths, None, '┌', '┬', '┐')];
    lines.extend(table_content_rows(
        prefix,
        &header,
        &widths,
        alignments,
        true,
        source_lines.first().cloned(),
    ));

    // A header-only table (no body rows) closes right after the header.
    if body.is_empty() {
        lines.push(table_rule(prefix, &widths, None, '└', '┴', '┘'));
    } else {
        lines.push(table_rule(prefix, &widths, None, '├', '┼', '┤'));
        for (i, row) in body.iter().enumerate() {
            // Skip the header line and the delimiter line to reach this row's source.
            let row_source = source_lines.get(i + 2).cloned();
            lines.extend(table_content_rows(
                prefix, row, &widths, alignments, false, row_source,
            ));
            let (left, junction, right) = if i + 1 == body.len() {
                ('└', '┴', '┘')
            } else {
                ('├', '┼', '┤')
            };
            lines.push(table_rule(prefix, &widths, None, left, junction, right));
        }
    }

    lines.push(empty_virtual_line!());
    lines
}

/// Flattens a row's cells to single-line strings, padding short rows out to `columns`.
fn table_cells(cells: &[RichText], columns: usize) -> Vec<String> {
    let mut row = cells
        .iter()
        .map(|cell| cell.to_string().replace('\n', " "))
        .collect::<Vec<_>>();
    row.resize(columns, String::new());
    row
}

/// Source range per line of the table's raw markdown, so display rows can map back to source.
fn table_source_lines(content: &str, source_range: &SourceRange<usize>) -> Vec<SourceRange<usize>> {
    let table = content.get(source_range.clone()).unwrap_or("");
    let mut start = source_range.start;
    table
        .split_inclusive('\n')
        .map(|line| {
            let range = start..start + line.len();
            start = range.end;
            range
        })
        .collect()
}

/// Picks a width per column. Each column wants its natural (longest cell) width but never less
/// than its longest word. When the natural widths fit `available` they are used as-is; otherwise
/// the spare width is shared out in proportion to each column's demand (`natural - minimum`), the
/// way browsers lay out tables — so a long column takes the most slack without starving the others.
fn table_column_widths(
    header: &[String],
    body: &[Vec<String>],
    columns: usize,
    available: usize,
) -> Vec<usize> {
    let mut natural = vec![MIN_COLUMN_WIDTH; columns];
    let mut minimum = vec![MIN_COLUMN_WIDTH; columns];

    for row in std::iter::once(header).chain(body.iter().map(Vec::as_slice)) {
        for (column, cell) in row.iter().enumerate() {
            let longest_word = cell
                .split_whitespace()
                .map(|w| w.width())
                .max()
                .unwrap_or(0);
            natural[column] = natural[column].max(cell.width());
            minimum[column] = minimum[column].max(longest_word);
        }
    }

    // One vertical border between and around columns, plus a space of padding each side.
    let chrome = (columns + 1) + 2 * columns;
    let budget = available.saturating_sub(chrome);

    if natural.iter().sum::<usize>() <= budget {
        return natural;
    }

    // Too wide: give every column its minimum, then share the surplus in proportion to demand.
    let demand = (0..columns)
        .map(|column| natural[column] - minimum[column])
        .collect::<Vec<_>>();
    let total_demand = demand.iter().sum::<usize>();
    let surplus = budget.saturating_sub(minimum.iter().sum());
    if total_demand == 0 || surplus == 0 {
        return minimum;
    }

    let mut widths = minimum;
    let mut remaining = surplus;
    for column in 0..columns {
        let share = surplus * demand[column] / total_demand;
        widths[column] += share;
        remaining -= share;
    }
    // Hand out the rounding remainder to the columns furthest from their natural width.
    while remaining > 0 {
        match (0..columns)
            .filter(|&column| widths[column] < natural[column])
            .max_by_key(|&column| natural[column] - widths[column])
        {
            Some(column) => {
                widths[column] += 1;
                remaining -= 1;
            }
            None => break,
        }
    }
    widths
}

/// Wraps a row's cells to their column widths, emitting one [`VirtualLine`] per wrapped text row.
fn table_content_rows<'a>(
    prefix: &Span<'static>,
    row: &[String],
    widths: &[usize],
    alignments: &[ast::Alignment],
    header: bool,
    source_range: Option<SourceRange<usize>>,
) -> Vec<VirtualLine<'a>> {
    let wrapped = row
        .iter()
        .zip(widths)
        .map(|(cell, &width)| wrap_preserve_trailing(cell, width, 0))
        .collect::<Vec<_>>();

    let height = wrapped.iter().map(Vec::len).max().unwrap_or(1).max(1);

    (0..height)
        .map(|line| {
            let mut spans = vec![synthetic_span!(prefix.clone())];
            // Anchor the row's source range on its first line (zero-width) so the
            // read-mode cursor and selection can resolve the table.
            if let (0, Some(range)) = (line, &source_range) {
                spans.push(content_span!("".to_string(), range.clone()));
            }
            spans.push(synthetic_span!(table_border("│")));
            for (column, &width) in widths.iter().enumerate() {
                // `wrap_preserve_trailing` keeps trailing spaces, which would over-fill a cell
                // and push the border out of alignment; cells don't need that fidelity.
                let text = wrapped[column]
                    .get(line)
                    .map(|cell| cell.trim_end())
                    .unwrap_or("");
                let alignment = alignments
                    .get(column)
                    .copied()
                    .unwrap_or(ast::Alignment::None);
                let cell = Span::raw(format!(" {} ", pad_cell(text, width, alignment)));
                spans.push(synthetic_span!(if header { cell.bold() } else { cell }));
                spans.push(synthetic_span!(table_border("│")));
            }
            VirtualLine::new(&spans)
        })
        .collect()
}

/// Builds a horizontal border line such as `├───┼───┤` spanning every column.
///
/// When `anchor` is set, a zero-width content span carries that source range so the line is a
/// navigation stop — used for the delimiter row, whose only rendering while editing is this
/// separator; without it the cursor could never reach the delimiter to edit it.
fn table_rule<'a>(
    prefix: &Span<'static>,
    widths: &[usize],
    anchor: Option<SourceRange<usize>>,
    left: char,
    junction: char,
    right: char,
) -> VirtualLine<'a> {
    let mut rule = String::from(left);
    for (column, &width) in widths.iter().enumerate() {
        if column > 0 {
            rule.push(junction);
        }
        rule.push_str(&"─".repeat(width + 2));
    }
    rule.push(right);

    let mut spans = vec![synthetic_span!(prefix.clone())];
    if let Some(range) = anchor {
        spans.push(content_span!("".to_string(), range));
    }
    spans.push(synthetic_span!(table_border(&rule)));
    VirtualLine::new(&spans)
}

/// Pads `content` to `width` display columns according to `alignment`.
fn pad_cell(content: &str, width: usize, alignment: ast::Alignment) -> String {
    let fill = width.saturating_sub(content.width());
    match alignment {
        ast::Alignment::Right => format!("{}{content}", " ".repeat(fill)),
        ast::Alignment::Center => {
            let left = fill / 2;
            format!("{}{content}{}", " ".repeat(left), " ".repeat(fill - left))
        }
        ast::Alignment::Left | ast::Alignment::None => format!("{content}{}", " ".repeat(fill)),
    }
}

fn table_border<'a>(text: &str) -> Span<'a> {
    Span::raw(text.to_string()).dark_gray()
}

// FIXME: Use options struct or similar
#[allow(clippy::too_many_arguments)]
pub fn render_node<'a>(
    content: String,
    node: &ast::Node,
    max_width: usize,
    horizontal_offset: usize,
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
            horizontal_offset,
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
            horizontal_offset,
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
            horizontal_offset,
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
            horizontal_offset,
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
            horizontal_offset,
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
            horizontal_offset,
            option,
            symbols,
        ),
        Table {
            alignments,
            head,
            rows,
            source_range,
        } => table(
            &content,
            prefix,
            alignments,
            head,
            rows,
            source_range,
            max_width,
            option,
            symbols,
        ),
    }
}
