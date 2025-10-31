use std::iter;

use ratatui::text::{Line, Span};

use crate::{
    note_editor::{
        ast::{self, SourceRange},
        editor::View,
        render::{render_node, text_wrap, RenderStyle},
    },
    stylized_text::{stylize, FontStyle},
};

macro_rules! content_span {
    ($span:expr, $range:expr) => {{
        VirtualSpan::Content($span, $range.clone())
    }};
}

macro_rules! synthetic_span {
    ($span:expr) => {{
        VirtualSpan::Synthetic($span.into())
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

    pub fn source_range(&self) -> Option<&SourceRange<usize>> {
        match self {
            Self::Content(.., source_range) => Some(source_range),
            Self::Synthetic(..) => None,
        }
    }

    /// Only content span width is taken into account when calculating width
    pub fn width(&self) -> usize {
        match self {
            VirtualSpan::Content(span, ..) => span.width(),
            _ => 0,
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

    /// Synthetic spans are not calculated into line width
    pub fn width(&self) -> usize {
        self.spans.iter().map(|span| span.width()).sum()
    }

    pub fn contains_offset(&self, offset: usize) -> bool {
        self.spans
            .iter()
            .any(|visual_span| visual_span.contains_offset(offset))
    }

    pub fn spans(self) -> Vec<Span<'a>> {
        self.spans
            .into_iter()
            .map(|visual_span| visual_span.into())
            .collect()
    }

    pub fn virtual_spans(self) -> Vec<VirtualSpan<'a>> {
        self.spans
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
        self.spans
            .iter()
            .any(|span| matches!(span, VirtualSpan::Content(..)))
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

    pub fn lines(&self) -> &[VirtualLine<'_>] {
        &self.lines
    }

    pub fn source_range(&self) -> &SourceRange<usize> {
        &self.source_range
    }
}

#[derive(Clone, Debug, Default)]
pub struct VirtualDocument<'a> {
    meta: Vec<VirtualLine<'a>>,
    blocks: Vec<VirtualBlock<'a>>,
    lines: Vec<VirtualLine<'a>>,
    line_to_block: Vec<usize>,
}

impl VirtualDocument<'_> {
    pub fn new() -> Self {
        Self::default()
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

    pub fn get_block(&self, line: usize) -> Option<(usize, &VirtualBlock<'_>)> {
        self.line_to_block().get(line).and_then(|block_idx| {
            self.blocks()
                .get(*block_idx)
                .map(|block| (*block_idx, block))
        })
    }

    pub fn layout(
        &mut self,
        note_name: &str,
        content: &str,
        view: &View,
        cursor_line: usize,
        ast_nodes: &[ast::Node],
        width: usize,
    ) {
        if !note_name.is_empty() {
            let mut meta = text_wrap(
                &stylize(note_name, FontStyle::BlackBoardBold).into(),
                Span::default(),
                &(0..1),
                width,
                None,
                &RenderStyle::Visual,
            );
            meta.extend([
                virtual_line!([synthetic_span!("═".repeat(width))]),
                empty_virtual_line!(),
            ]);

            self.meta = meta;
        }

        let current_block_idx = self.line_to_block().get(cursor_line);

        let (blocks, lines, line_to_block) = ast_nodes.iter().enumerate().fold(
            (vec![], vec![], vec![]),
            |(mut blocks, mut lines, mut line_to_block), (idx, node)| {
                let block = if current_block_idx
                    .is_some_and(|block_idx| *block_idx == idx && matches!(view, View::Edit(..)))
                {
                    render_node(
                        content.to_string(),
                        node,
                        width,
                        Span::default(),
                        &RenderStyle::Raw,
                    )
                } else {
                    render_node(
                        content.to_string(),
                        node,
                        width,
                        Span::default(),
                        &RenderStyle::Visual,
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
    }
}
