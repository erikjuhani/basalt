use ratatui::{
    style::{Color, Modifier, Style, Stylize},
    text::{Span, ToSpan},
};

use crate::{
    note_editor::{
        ast::{self, SourceRange},
        rich_text::RichText,
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
    let prefix_width = prefix.width();
    let wrap_marker = "⤷ ";

    let options = textwrap::Options::new(width.saturating_sub(prefix_width + wrap_marker.len()))
        .break_words(false);

    textwrap::wrap(text.content.trim_end(), &options)
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                match &marker {
                    Some(marker) if *option == RenderStyle::Visual => virtual_line!([
                        synthetic_span!(prefix.clone()),
                        synthetic_span!(marker.clone()),
                        content_span!(Span::styled(line.to_string(), text.style), source_range)
                    ]),
                    _ => virtual_line!([
                        synthetic_span!(prefix.clone()),
                        content_span!(Span::styled(line.to_string(), text.style), source_range)
                    ]),
                }
            } else {
                virtual_line!([
                    synthetic_span!(prefix.clone()),
                    synthetic_span!(Span::styled(
                        " ".repeat(prefix_width.saturating_sub(1).max(1)),
                        prefix.style
                    )),
                    synthetic_span!(Span::styled(wrap_marker, Style::new().black())),
                    content_span!(Span::styled(line.to_string(), text.style), source_range),
                ])
            }
        })
        .collect()
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

    let text = text.to_string();
    let text = match option {
        RenderStyle::Visual => text,
        RenderStyle::Raw => content
            .get(source_range.clone())
            .map_or(text, |source| source.into()),
    };
    let prefix_width = prefix.width();

    macro_rules! heading {
        (_, $text:expr, $underline:expr) => {{
            heading!("", $text, $underline)
        }};

        ($prefix: expr, $text:expr, $underline:expr) => {{
            let mut wrapped_heading = text_wrap(
                &$text,
                prefix.clone(),
                source_range,
                max_width,
                Some($prefix.into()),
                option,
            );
            wrapped_heading.extend([virtual_line!([
                synthetic_span!(prefix),
                synthetic_span!($underline)
            ])]);
            wrapped_heading
        }};

        ($prefix:expr, $text:expr) => {{
            Self::text_wrap(&$text.into(), $prefix.into(), source_range, max_width)
        }};
    }

    let lines = match level {
        H1 => heading!(
            _,
            text.to_uppercase().bold(),
            "═".repeat(max_width.saturating_sub(prefix_width))
        ),
        H2 => heading!(
            _,
            text.bold().yellow(),
            "─".repeat(max_width.saturating_sub(prefix_width)).yellow()
        ),
        H3 => heading!("⬤  ".cyan(), text.bold().cyan(), Span::default()),
        H4 => heading!("● ".magenta(), text.bold().magenta(), Span::default()),
        H5 => heading!(
            "◆ ".to_span(),
            stylize(&text, FontStyle::Script).into(),
            Span::default()
        ),
        H6 => heading!(
            "✺ ".to_span(),
            stylize(&text, FontStyle::Script).into(),
            Span::default()
        ),
    };

    VirtualBlock::new(&lines, source_range)
}

pub fn paragraph<'a>(
    content: &str,
    prefix: Span<'static>,
    text: &RichText,
    source_range: &SourceRange<usize>,
    max_width: usize,
    option: &RenderStyle,
) -> VirtualBlock<'a> {
    let text = text.to_string();
    let text = match option {
        RenderStyle::Visual => text,
        RenderStyle::Raw => content
            .get(source_range.clone())
            .map_or(text, |source| source.into()),
    };

    let mut lines = text_wrap(
        &text.into(),
        prefix.clone(),
        source_range,
        max_width,
        None,
        option,
    );

    if prefix.to_string().is_empty() {
        lines.extend([empty_virtual_line!()]);
    }

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
    let text = text.to_string();
    let text = match option {
        RenderStyle::Visual => text,
        RenderStyle::Raw => content
            .get(source_range.clone())
            .map_or(text, |source| source.into()),
    };

    let padding_line = virtual_line!([
        synthetic_span!(prefix.clone()),
        synthetic_span!(" "
            .repeat(max_width.saturating_sub(prefix.width()))
            .bg(Color::Black))
    ]);

    let mut lines = vec![padding_line.clone()];
    lines.extend(text.lines().map(|line| {
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
    let mut lines: Vec<VirtualLine<'a>> = nodes
        .iter()
        .flat_map(|node| {
            render_node(content.to_string(), node, max_width, prefix.clone(), option).lines
        })
        .collect();

    if prefix.to_string().is_empty() {
        lines.extend([empty_virtual_line!()]);
    }

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
    let Some(text) = nodes.first().and_then(|first| first.rich_text()) else {
        return VirtualBlock::new(&[], source_range);
    };

    let text = text.to_string();
    let text = match option {
        RenderStyle::Visual => text,
        RenderStyle::Raw => content
            .get(source_range.clone())
            .map_or(text, |source| source.into()),
    };
    let (marker, text) = match kind {
        ast::TaskKind::Unchecked => ("□ ".dark_gray(), text.into()),
        ast::TaskKind::LooselyChecked => ("■ ".magenta(), text.dark_gray()),
        ast::TaskKind::Checked => (
            "■ ".magenta(),
            text.dark_gray().add_modifier(Modifier::CROSSED_OUT),
        ),
    };

    let lines = text_wrap(
        &text,
        prefix.clone(),
        source_range,
        max_width,
        Some(marker),
        option,
    );

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
    let Some((text, _first, rest)) = nodes.split_first().and_then(|(first, rest)| {
        let text = first.rich_text()?;
        Some((text, first, rest))
    }) else {
        return VirtualBlock::new(&[], source_range);
    };

    let text = text.to_string();
    let marker = match kind {
        ast::ItemKind::Ordered(i) => format!("{i}. ").dark_gray(),
        ast::ItemKind::Unordered => "- ".dark_gray(),
    };

    let text = match option {
        RenderStyle::Visual => text,
        RenderStyle::Raw => content
            .get(source_range.clone())
            .map_or(text, |source| source.into()),
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
            prefix.clone().merge("  ".into()),
            option,
        )
        .lines
    }));

    VirtualBlock::new(&lines, source_range)
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
    let lines: Vec<VirtualLine<'a>> = nodes
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
        .collect();

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
