use std::{
    collections::HashMap,
    iter,
    str::{CharIndices, Chars},
};

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

use std::borrow::Cow;

use crate::{
    config::Symbols,
    image::ImageKey,
    note_editor::{
        ast::{self, ImageSource, SourceRange},
        render::{edit_lines, render_node, text_wrap, trailing_empty_lines, RenderStyle},
        state::View,
        text_buffer::TextBuffer,
    },
    stylized_text::stylize,
};

/// Upper bound on the on-screen height of an image, so a tall picture cannot
/// take over the whole viewport.
const MAX_IMAGE_ROWS: u16 = 20;

/// Rows reserved for an image whose dimensions are not yet known (still loading
/// or could not be decoded).
const PLACEHOLDER_ROWS: u16 = 3;

/// Per-note image data used during layout: how each image source resolves, the
/// pixel dimensions of any decoded images, and the terminal's font-cell size.
/// Populated by the app each frame from the [`crate::image::ImageStore`]; kept
/// here (rather than the heavy store) so [`super::state::NoteEditorState`] stays
/// cheap to clone.
#[derive(Clone, Debug, Default)]
pub struct ImageContext {
    pub resolved: HashMap<ImageSource, ImageKey>,
    pub dims: HashMap<ImageKey, (u32, u32)>,
    pub font_size: (u16, u16),
}

impl ImageContext {
    /// Resolved key and reserved cell size for a block image, or `None` when the
    /// source cannot be resolved (the caller then renders a not-found marker).
    pub fn placement(
        &self,
        source: &ImageSource,
        max_width: usize,
    ) -> Option<(ImageKey, (u16, u16))> {
        let key = self.resolved.get(source)?.clone();
        let size = match self.dims.get(&key) {
            Some(&dims) => cell_size(dims, self.font_size, max_width),
            None => ((max_width.min(40)) as u16, PLACEHOLDER_ROWS),
        };
        Some((key, size))
    }
}

/// Fits an image's pixel dimensions into terminal cells, capped to the content
/// width and [`MAX_IMAGE_ROWS`] while preserving aspect ratio.
fn cell_size(
    (pixel_width, pixel_height): (u32, u32),
    (fw, fh): (u16, u16),
    max_width: usize,
) -> (u16, u16) {
    let fw = fw.max(1) as u32;
    let fh = fh.max(1) as u32;
    let mut cols = (pixel_width / fw).max(1);
    let mut rows = (pixel_height / fh).max(1);

    let max_cols = max_width.max(1) as u32;
    if cols > max_cols {
        rows = (rows * max_cols / cols).max(1);
        cols = max_cols;
    }
    let max_rows = MAX_IMAGE_ROWS as u32;
    if rows > max_rows {
        cols = (cols * max_rows / rows).max(1);
        rows = max_rows;
    }
    (cols as u16, rows as u16)
}

/// A laid-out image: where its reserved rows begin in the document and the
/// resolved key the overlay pass draws from.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePlacement {
    /// Index into [`VirtualDocument::lines`] of the image's first reserved row.
    pub doc_line: usize,
    pub width: u16,
    pub height: u16,
    pub key: ImageKey,
}

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
        let span = match self {
            VirtualSpan::Content(span, ..) | VirtualSpan::Synthetic(span) => span,
        };
        // A tab is one byte but rendered as two columns (expanded at draw time).
        span.content
            .chars()
            .map(|c| if c == '\t' { 2 } else { c.width().unwrap_or(0) })
            .sum()
    }

    pub fn is_synthetic(&self) -> bool {
        matches!(self, VirtualSpan::Synthetic(..))
    }
}

impl<'a> From<VirtualSpan<'a>> for Span<'a> {
    fn from(value: VirtualSpan<'a>) -> Self {
        let span = match value {
            VirtualSpan::Synthetic(span) | VirtualSpan::Content(span, _) => span,
        };
        // Expand tabs so the terminal doesn't break the layout; the cursor maps
        // by byte offset against the un-expanded content (tabs counted as two).
        match span.content.contains('\t') {
            true => Span::styled(span.content.replace('\t', "  "), span.style),
            false => span,
        }
    }
}

/// Tags the top reserved row of a block image so its placement can be recovered
/// by scanning the laid-out lines, regardless of how deeply the image is nested.
#[derive(Clone, PartialEq, Debug)]
pub struct ImageMark {
    pub key: ImageKey,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, PartialEq, Debug)]
pub struct VirtualLine<'a> {
    spans: Vec<VirtualSpan<'a>>,
    image: Option<ImageMark>,
}

impl<'a> VirtualLine<'a> {
    pub fn new(spans: &[VirtualSpan<'a>]) -> Self {
        VirtualLine {
            spans: spans.to_vec(),
            image: None,
        }
    }

    pub fn with_image(mut self, mark: ImageMark) -> Self {
        self.image = Some(mark);
        self
    }

    pub fn image_mark(&self) -> Option<&ImageMark> {
        self.image.as_ref()
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

fn is_empty_line(line: &VirtualLine<'_>) -> bool {
    line.virtual_spans()
        .iter()
        .all(|span| span.is_synthetic() && span.width() == 0)
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
    meta: Vec<VirtualLine<'a>>,
    blocks: Vec<VirtualBlock<'a>>,
    lines: Vec<VirtualLine<'a>>,
    line_to_block: Vec<usize>,
    image_placements: Vec<ImagePlacement>,
}

impl<'a> VirtualDocument<'a> {
    pub fn new(symbols: &Symbols) -> Self {
        Self {
            symbols: symbols.clone(),
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

    pub fn image_placements(&self) -> &[ImagePlacement] {
        &self.image_placements
    }

    pub fn line_to_block_idx(&self, line: usize) -> usize {
        self.line_to_block.get(line).cloned().unwrap_or(0)
    }

    pub fn get_block(&self, block_idx: usize) -> Option<(usize, &VirtualBlock<'_>)> {
        self.blocks().get(block_idx).map(|block| (block_idx, block))
    }

    // FIXME: Refactor. Too many arguments.
    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        &mut self,
        note_name: &str,
        content: &str,
        view: &View,
        current_block_idx: Option<usize>,
        cursor_offset: usize,
        ast_nodes: &[ast::Node],
        width: usize,
        text_buffer: Option<TextBuffer>,
        images: &ImageContext,
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
                &RenderStyle::Reader,
                &self.symbols,
            );
            meta.extend([
                virtual_line!([synthetic_span!(self.symbols.horizontal_rule.repeat(width))]),
                empty_virtual_line!(),
            ]);

            self.meta = meta;
        }

        let styled = match view {
            View::Edit(..) => RenderStyle::Visual,
            View::Read => RenderStyle::Reader,
        };
        let live_content: Cow<'_, str> = text_buffer
            .as_ref()
            .filter(|tb| tb.modified)
            .map(|tb| Cow::Owned(tb.write(content)))
            .unwrap_or(Cow::Borrowed(content));

        let (blocks, lines, line_to_block) = ast_nodes.iter().enumerate().fold(
            (vec![], vec![], vec![]),
            |(mut blocks, mut lines, mut line_to_block), (idx, node)| {
                let is_active = current_block_idx == Some(idx) && matches!(view, View::Edit(..));

                // The active block reads from the edit buffer, which may not yet
                // be re-parsed. Its range tracks in-flight edits exactly.
                let active_range = is_active
                    .then(|| {
                        text_buffer
                            .as_ref()
                            .map(|buffer| buffer.source_range.clone())
                    })
                    .flatten();

                let mut block = match &active_range {
                    // The active block is rendered line by line from its edit
                    // buffer: the cursor's line raw, the rest decorated in place.
                    // This keeps a 1:1 source/display mapping, so nested lists and
                    // structural edits stay reliable regardless of stale ast_nodes.
                    Some(range) => {
                        let buffer_content = text_buffer
                            .as_ref()
                            .map(|b| b.content.as_str())
                            .unwrap_or("");
                        let lines = edit_lines(
                            buffer_content,
                            range.start,
                            cursor_offset,
                            width,
                            &self.symbols,
                        );
                        VirtualBlock::new(&lines, range)
                    }
                    None => render_node(
                        live_content.to_string(),
                        node,
                        width,
                        Span::default(),
                        &styled,
                        &self.symbols,
                        0,
                        images,
                    ),
                };

                if matches!(styled, RenderStyle::Visual) {
                    // Rendering may emit its own trailing blanks; drop them so
                    // spacing is derived purely from the source below.
                    while block.lines.last().is_some_and(is_empty_line) {
                        block.lines.pop();
                    }

                    let block_range = active_range
                        .clone()
                        .unwrap_or_else(|| node.source_range().clone());
                    let slice = live_content.get(block_range.clone()).unwrap_or("");
                    let end = block_range.end;

                    // Append empty rows so on-screen spacing mirrors the source:
                    // blanks already inside the block, plus blanks in the gap to
                    // the next block. The last block gets one trailing row.
                    let trailing = match ast_nodes.get(idx + 1) {
                        None => 1,
                        Some(next) => {
                            let absorbed = if is_active {
                                0
                            } else {
                                trailing_empty_lines(slice)
                            };
                            let gap = live_content
                                .get(end..next.source_range().start)
                                .unwrap_or("");
                            let gap_blanks = gap.bytes().filter(|byte| *byte == b'\n').count();
                            // The first newline only terminates the block's last
                            // line unless that line already ended with one.
                            let terminator = !slice.ends_with('\n') as usize;
                            absorbed + gap_blanks.saturating_sub(terminator)
                        }
                    };

                    block
                        .lines
                        .extend((0..trailing).map(|_| empty_virtual_line!()));
                }

                let block_lines = block.lines.clone();
                let line_count = block_lines.len();

                blocks.push(block);
                lines.extend(block_lines);
                line_to_block.extend(iter::repeat_n(idx, line_count));

                (blocks, lines, line_to_block)
            },
        );

        // Image placements are recovered by scanning the laid-out lines for the
        // marks left by `render::image`, so images at any nesting depth (e.g.
        // under list items) are picked up uniformly.
        self.image_placements = lines
            .iter()
            .enumerate()
            .filter_map(|(doc_line, line)| {
                line.image_mark().map(|mark| ImagePlacement {
                    doc_line,
                    width: mark.width,
                    height: mark.height,
                    key: mark.key.clone(),
                })
            })
            .collect();

        self.blocks = blocks;
        self.lines = lines;
        self.line_to_block = line_to_block;
    }
}
