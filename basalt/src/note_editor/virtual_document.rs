use std::{
    collections::{HashMap, HashSet},
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
        ast::{self, ImageSize, ImageSource, SourceRange},
        render::{
            edit_lines, edit_table, render_node, text_wrap, trailing_empty_lines, RenderStyle,
        },
        state::View,
        text_buffer::TextBuffer,
    },
    stylized_text::stylize,
};

/// Default until the app applies the configured ratio.
const DEFAULT_IMAGE_MAX_HEIGHT_RATIO: f32 = 0.8;

/// Rows reserved while an image's dimensions are still unknown.
const PLACEHOLDER_ROWS: u16 = 3;

/// Logical (CSS) width of a monospace cell, used to read the display scale off
/// the reported device cell size so Obsidian's logical pixel sizes match.
const LOGICAL_CELL_WIDTH: f32 = 8.0;

/// Per-note image sizing data, populated by the app from [`crate::image::ImageStore`].
/// Kept out of the heavy store so [`super::state::NoteEditorState`] stays cheap to clone.
#[derive(Clone, Debug)]
pub struct ImageContext {
    pub resolved: HashMap<ImageSource, ImageKey>,
    pub dims: HashMap<ImageKey, (u32, u32)>,
    /// Sources the store could not resolve or decode. An unresolved source not
    /// in here is still loading, so it shows a placeholder rather than an error.
    pub failed: HashSet<ImageSource>,
    pub font_size: (u16, u16),
    /// Max image height as a fraction of the viewport height.
    pub max_height_ratio: f32,
    /// Row cap derived from `max_height_ratio` and the viewport height.
    pub max_rows: u16,
}

impl Default for ImageContext {
    fn default() -> Self {
        Self {
            resolved: HashMap::new(),
            dims: HashMap::new(),
            failed: HashSet::new(),
            font_size: (0, 0),
            max_height_ratio: DEFAULT_IMAGE_MAX_HEIGHT_RATIO,
            max_rows: 0,
        }
    }
}

impl ImageContext {
    /// Refreshes [`Self::max_rows`] for the current viewport height.
    pub fn set_viewport_height(&mut self, height: u16) {
        self.max_rows = (height as f32 * self.max_height_ratio).round() as u16;
    }

    /// Resolved key and reserved cell size for a block image, or `None` while the
    /// source is unresolved (still loading or failed).
    pub fn placement(
        &self,
        source: &ImageSource,
        size: Option<ImageSize>,
        max_width: usize,
    ) -> Option<(ImageKey, (u16, u16))> {
        let key = self.resolved.get(source)?.clone();
        let cell = match self.dims.get(&key) {
            Some(&dims) => cell_size(dims, self.font_size, max_width, self.max_rows, size),
            None => ((max_width.min(40)) as u16, PLACEHOLDER_ROWS),
        };
        Some((key, cell))
    }

    /// Whether an unresolved source has failed (versus still loading).
    pub fn is_failed(&self, source: &ImageSource) -> bool {
        self.failed.contains(source)
    }
}

/// Scales an image's pixel dimensions to fit its box, up or down, preserving
/// aspect ratio. The box fills the available width unless an explicit pixel
/// `size` caps it (Obsidian `|width` / `|widthxheight`); the viewport row cap
/// always applies.
fn cell_size(
    (pixel_width, pixel_height): (u32, u32),
    (fw, fh): (u16, u16),
    max_width: usize,
    max_rows: u16,
    size: Option<ImageSize>,
) -> (u16, u16) {
    let natural_cols = (pixel_width as f32 / fw.max(1) as f32).max(1.0);
    let natural_rows = (pixel_height as f32 / fh.max(1) as f32).max(1.0);

    // Obsidian sizes are logical pixels but the cell size is reported in device
    // pixels, so on a HiDPI display an explicit size must scale up to match.
    let display_scale = (fw.max(1) as f32 / LOGICAL_CELL_WIDTH).round().max(1.0);
    let max_cols = match size {
        Some(size) => (size.width as f32 * display_scale / fw.max(1) as f32).min(max_width as f32),
        None => max_width as f32,
    }
    .max(1.0);
    let max_rows = match size.and_then(|size| size.height) {
        Some(height) => (height as f32 * display_scale / fh.max(1) as f32).min(max_rows as f32),
        None => max_rows as f32,
    }
    .max(1.0);

    let scale = (max_cols / natural_cols).min(max_rows / natural_rows);
    let cols = (natural_cols * scale).round().max(1.0) as u16;
    let rows = (natural_rows * scale).round().max(1.0) as u16;
    (cols, rows)
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
        horizontal_offset: usize,
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
                virtual_line!([synthetic_span!(self
                    .symbols
                    .horizontal_rule
                    .repeat(width + horizontal_offset))]),
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
                        // A table edits as a box with the cursor's row revealed raw;
                        // every other block edits raw line by line.
                        let lines = if matches!(node, ast::Node::Table { .. }) {
                            edit_table(
                                buffer_content,
                                range.start,
                                cursor_offset,
                                width,
                                horizontal_offset,
                                &self.symbols,
                            )
                        } else {
                            edit_lines(
                                buffer_content,
                                range.start,
                                cursor_offset,
                                width,
                                horizontal_offset,
                                &self.symbols,
                            )
                        };
                        VirtualBlock::new(&lines, range)
                    }
                    None => render_node(
                        live_content.to_string(),
                        node,
                        width,
                        horizontal_offset,
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

        // Scan the laid-out lines for image marks, catching any nesting depth.
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

#[cfg(test)]
mod tests {
    use super::*;

    // 800x400px image, 10x20px font cells: 80x20 natural cells, 2:1 aspect.
    const DIMS: (u32, u32) = (800, 400);
    const FONT: (u16, u16) = (10, 20);

    #[test]
    fn unsized_image_fills_the_width() {
        assert_eq!(cell_size(DIMS, FONT, 40, 30, None), (40, 10));
    }

    #[test]
    fn explicit_width_caps_the_box_and_preserves_aspect() {
        // 200px wide at a 10px cell is 20 cols; height follows the 2:1 aspect.
        let size = Some(ImageSize {
            width: 200,
            height: None,
        });
        assert_eq!(cell_size(DIMS, FONT, 40, 30, size), (20, 5));
    }

    #[test]
    fn explicit_width_scales_up_on_a_hidpi_cell() {
        // A 16px device cell reads as a 2x display, so 200 logical px becomes
        // 400 device px = 25 cols (vs 12 without the scale).
        let size = Some(ImageSize {
            width: 200,
            height: None,
        });
        assert_eq!(cell_size(DIMS, (16, 32), 80, 30, size).0, 25);
    }

    #[test]
    fn explicit_width_never_exceeds_the_available_width() {
        let size = Some(ImageSize {
            width: 9999,
            height: None,
        });
        assert_eq!(cell_size(DIMS, FONT, 40, 30, size).0, 40);
    }

    #[test]
    fn explicit_height_bounds_the_box() {
        // 100px tall at a 20px cell is 5 rows; width follows the 4:1 cell aspect.
        let size = Some(ImageSize {
            width: 9999,
            height: Some(100),
        });
        assert_eq!(cell_size(DIMS, FONT, 40, 30, size), (20, 5));
    }
}
