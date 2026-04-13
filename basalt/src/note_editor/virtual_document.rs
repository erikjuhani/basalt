use std::{
    iter,
    str::{CharIndices, Chars},
};

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::{
    app::SyntectContext,
    config::Symbols,
    note_editor::{
        ast::{self, SourceRange},
        render::{render_node, text_wrap, RenderStyle},
        state::View,
        text_buffer::TextBuffer,
        rich_text::LinkMapEntry,
    },
    stylized_text::stylize,
};

macro_rules! content_span {
    ($span:expr, $range:expr) => {{
        VirtualSpan::Content($span.into(), $range.clone())
    }};
}

macro_rules! synthetic_span {
    ($span:expr) => {{
        VirtualSpan::Synthetic($span.clone().into())
    }};
}

macro_rules! virtual_line {
    ($visual_spans:expr) => {{
        VirtualLine::new(&$visual_spans)
    }};
}

macro_rules! empty_virtual_line {
    () => {{
        VirtualLine::new(&[synthetic_span!(Span::default())])
    }};
}

pub(crate) use content_span;
pub(crate) use empty_virtual_line;
pub(crate) use synthetic_span;
pub(crate) use virtual_line;

#[derive(Clone, PartialEq, Debug)]
pub enum VirtualSpan<'a> {
    Synthetic(Span<'a>),
    Content(Span<'a>, SourceRange<usize>),
}

impl VirtualSpan<'_> {
    pub fn contains_offset(&self, offset: usize) -> bool {
        match self {
            VirtualSpan::Content(_, source_range) => source_range.contains(&offset),
            _ => false,
        }
    }

    pub fn chars(&self) -> Chars<'_> {
        match self {
            Self::Content(span, ..) => span.content.chars(),
            Self::Synthetic(..) => "".chars(),
        }
    }

    pub fn char_indices(&self) -> CharIndices<'_> {
        match self {
            Self::Content(span, ..) => span.content.char_indices(),
            Self::Synthetic(..) => "".char_indices(),
        }
    }

    pub fn source_range(&self) -> Option<&SourceRange<usize>> {
        match self {
            Self::Content(.., source_range) => Some(source_range),
            Self::Synthetic(..) => None,
        }
    }

    pub fn width(&self) -> usize {
        match self {
            VirtualSpan::Content(span, ..) | VirtualSpan::Synthetic(span) => span.width(),
        }
    }

    pub fn is_synthetic(&self) -> bool {
        matches!(self, VirtualSpan::Synthetic(..))
    }
}

impl<'a> From<VirtualSpan<'a>> for Span<'a> {
    fn from(value: VirtualSpan<'a>) -> Self {
        match value {
            VirtualSpan::Synthetic(span) => span,
            VirtualSpan::Content(span, _) => span,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct VirtualLine<'a> {
    spans: Vec<VirtualSpan<'a>>,
}

impl<'a> VirtualLine<'a> {
    pub fn new(spans: &[VirtualSpan<'a>]) -> Self {
        VirtualLine {
            spans: spans.to_vec(),
        }
    }

    pub fn spans(self) -> Vec<Span<'a>> {
        self.spans.into_iter().map(|s| s.into()).collect()
    }

    pub fn virtual_spans(&self) -> &[VirtualSpan<'a>] {
        &self.spans
    }

    pub fn source_range(&self) -> Option<SourceRange<usize>> {
        self.spans
            .iter()
            .fold(None, |acc: Option<(usize, usize)>, span| {
                if let Some(source_range) = span.source_range() {
                    Some(
                        acc.map_or((source_range.start, source_range.end), |(start, _)| {
                            (start, source_range.end)
                        }),
                    )
                } else {
                    acc
                }
            })
            .map(|(start, end)| start..end)
    }

    pub fn has_content(&self) -> bool {
        // We short-circuit when we find content span
        self.spans.iter().any(|span| !span.is_synthetic())
    }
}

impl<'a> From<VirtualLine<'a>> for Line<'a> {
    fn from(val: VirtualLine<'a>) -> Self {
        Line::from(val.spans())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualBlock<'a> {
    pub lines: Vec<VirtualLine<'a>>,
    pub source_range: SourceRange<usize>,
}

impl<'a> VirtualBlock<'a> {
    pub fn new(lines: &[VirtualLine<'a>], source_range: &SourceRange<usize>) -> Self {
        Self {
            lines: lines.to_vec(),
            source_range: source_range.clone(),
        }
    }

    pub fn source_range(&self) -> &SourceRange<usize> {
        &self.source_range
    }
}

#[derive(Clone, Debug, Default)]
pub struct VirtualDocument<'a> {
    symbols: Symbols,
    syntect_ctx: Option<SyntectContext>,
    meta: Vec<VirtualLine<'a>>,
    blocks: Vec<VirtualBlock<'a>>,
    lines: Vec<VirtualLine<'a>>,
    line_to_block: Vec<usize>,
    link_map: Vec<LinkMapEntry>,
}

impl<'a> VirtualDocument<'a> {
    pub fn new(symbols: &Symbols, syntect_ctx: Option<&SyntectContext>) -> Self {
        Self {
            symbols: symbols.clone(),
            syntect_ctx: syntect_ctx.cloned(),
            ..Default::default()
        }
    }
    pub fn meta(&self) -> &[VirtualLine<'_>] {
        &self.meta
    }

    pub fn blocks(&self) -> &[VirtualBlock<'_>] {
        &self.blocks
    }

    pub fn lines(&self) -> &[VirtualLine<'_>] {
        &self.lines
    }

    pub fn line_to_block(&self) -> &[usize] {
        &self.line_to_block
    }

    pub fn link_map(&self) -> &[LinkMapEntry] {
        &self.link_map
    }

    pub fn get_block(&self, block_idx: usize) -> Option<(usize, &VirtualBlock<'_>)> {
        self.blocks().get(block_idx).map(|block| (block_idx, block))
    }

    pub fn syntect_selection_color(&self) -> Option<ratatui::style::Color> {
        self.syntect_ctx
            .as_ref()
            .and_then(|ctx| ctx.selection_color)
    }

    // FIXME: Refactor. Too many arguments.
    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        &mut self,
        note_name: &str,
        content: &str,
        view: &View,
        current_block_idx: Option<usize>,
        ast_nodes: &[ast::Node],
        width: usize,
        text_buffer: Option<TextBuffer>,
        // Horizontal scroll offset for the currently active table (D-15).
        // Passed from `NoteEditorState::table_h_scroll` at layout time.
        table_h_scroll: usize,
    ) {
        if !note_name.is_empty() {
            let note_name = match self.symbols.title_font_style {
                Some(style) => stylize(note_name, style),
                None => note_name.to_string(),
            };
            let mut meta = text_wrap(
                &Span::from(note_name),
                Span::default(),
                &(0..1),
                width,
                None,
                &RenderStyle::Visual,
                &self.symbols,
            );
            meta.extend([
                virtual_line!([synthetic_span!(self.symbols.horizontal_rule.repeat(width))]),
                empty_virtual_line!(),
            ]);

            self.meta = meta;
        }

        let (blocks, lines, line_to_block) = ast_nodes.iter().enumerate().fold(
            (vec![], vec![], vec![]),
            |(mut blocks, mut lines, mut line_to_block), (idx, node)| {
                let block = if current_block_idx
                    .is_some_and(|block_idx| block_idx == idx && matches!(view, View::Edit(..)))
                {
                    let mut node = node.clone();
                    if let Some(text_buffer) = &text_buffer {
                        node.set_source_range(text_buffer.source_range.clone());
                    }

                    render_node(
                        text_buffer
                            .clone()
                            .map(|text_buffer| text_buffer.content)
                            .unwrap_or_default(),
                        &node,
                        width,
                        Span::default(),
                        &RenderStyle::Raw,
                        &self.symbols,
                        0,
                        None, // No syntax highlighting in Raw/Edit mode
                        0,    // Raw/Edit mode: no table scroll
                    )
                } else {
                    render_node(
                        content.to_string(),
                        node,
                        width,
                        Span::default(),
                        &RenderStyle::Visual,
                        &self.symbols,
                        0,
                        self.syntect_ctx.as_ref(), // Pass syntect context for Visual rendering
                        table_h_scroll,            // Apply horizontal scroll for tables (D-15)
                    )
                };
                let block_lines = block.lines.clone();
                let line_count = block_lines.len();

                blocks.push(block);
                lines.extend(block_lines);
                line_to_block.extend(iter::repeat_n(idx, line_count));

                (blocks, lines, line_to_block)
            },
        );

        self.blocks = blocks;
        self.lines = lines;
        self.line_to_block = line_to_block;

        // Populate link_map for OSC 8 hyperlink emission
        self.link_map.clear();

        // Step A: Collect all external links from the AST in document order.
        let mut all_links: Vec<(String, String)> = Vec::new();
        fn collect_links(
            nodes: &[crate::note_editor::ast::Node],
            links: &mut Vec<(String, String)>,
        ) {
            for node in nodes {
                match node {
                    crate::note_editor::ast::Node::Paragraph { text, .. }
                    | crate::note_editor::ast::Node::Heading { text, .. } => {
                        for inline in text.nodes() {
                            if let crate::note_editor::rich_text::InlineNode::Link {
                                text,
                                target: crate::note_editor::rich_text::LinkTarget::External(
                                    url,
                                ),
                            } = inline
                            {
                                links.push((text.clone(), url.clone()));
                            }
                        }
                    }
                    crate::note_editor::ast::Node::BlockQuote { nodes, .. }
                    | crate::note_editor::ast::Node::List { nodes, .. }
                    | crate::note_editor::ast::Node::Item { nodes, .. }
                    | crate::note_editor::ast::Node::Task { nodes, .. } => {
                        collect_links(nodes, links);
                    }
                    _ => {}
                }
            }
        }
        collect_links(ast_nodes, &mut all_links);

        // Step B: Walk rendered lines and match link text to display positions.
        let mut link_iter = all_links.iter();
        'outer: for (line_idx, line) in self.lines.iter().enumerate() {
            let mut col = 0usize;
            for span in line.virtual_spans() {
                let s = match span {
                    VirtualSpan::Synthetic(s) | VirtualSpan::Content(s, _) => s,
                };
                let content_str = s.content.as_ref();
                let span_width = UnicodeWidthStr::width(content_str);

                if let Some((link_text, _url)) = link_iter.as_slice().first() {
                    if content_str == link_text.as_str() {
                        let (text, url) = link_iter.next().unwrap();
                        self.link_map.push(crate::note_editor::rich_text::LinkMapEntry {
                            line_idx,
                            col_start: col,
                            col_end: col + span_width,
                            text: text.clone(),
                            target: crate::note_editor::rich_text::LinkTarget::External(
                                url.clone(),
                            ),
                        });
                        if link_iter.as_slice().is_empty() {
                            break 'outer;
                        }
                    }
                }
                col += span_width;
            }
        }
    }
}
