use std::{
    collections::{hash_map::Entry, HashMap},
    hash::{DefaultHasher, Hash, Hasher},
    iter,
    str::{CharIndices, Chars},
};

use ratatui::{
    layout::Size,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthChar;

use std::borrow::Cow;

use crate::{
    config::{ImageMaxHeight, Symbols, Theme},
    image::{ImageKey, PixelSize},
    note_editor::{
        ast::{self, ImageSize, ImageSource, SourceRange},
        highlight,
        render::{
            edit_lines, edit_table, render_node, text_wrap, trailing_empty_lines, RenderContext,
            RenderStyle,
        },
        state::View,
        text_buffer::TextBuffer,
    },
    stylized_text::stylize,
};

const DEFAULT_FONT_SIZE: Size = Size::new(10, 20);

/// Logical width of a monospace cell, to read the HiDPI scale off the device cell.
const LOGICAL_CELL_WIDTH: f32 = 8.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageEntry {
    Loading,
    Failed,
    Decoded { key: ImageKey, size: PixelSize },
}

#[derive(Clone, Debug)]
pub enum Placement {
    Image { key: ImageKey, size: Size },
    Loading,
    Failed,
}

/// `generation` bumps on a real change, so the layout cache sees new image state.
#[derive(Clone, Debug)]
pub struct ImageContext {
    entries: HashMap<ImageSource, ImageEntry>,
    generation: u64,
    font_size: Size,
    max_height: ImageMaxHeight,
    max_rows: u16,
}

impl Default for ImageContext {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            generation: 0,
            font_size: DEFAULT_FONT_SIZE,
            max_height: ImageMaxHeight::default(),
            max_rows: 0,
        }
    }
}

impl ImageContext {
    pub fn set_meta(&mut self, font_size: Size, max_height: ImageMaxHeight) {
        if (font_size, max_height) != (self.font_size, self.max_height) {
            self.font_size = font_size;
            self.max_height = max_height;
            self.generation = self.generation.wrapping_add(1);
        }
    }

    pub fn set_entries(&mut self, entries: HashMap<ImageSource, ImageEntry>) {
        if entries != self.entries {
            self.entries = entries;
            self.generation = self.generation.wrapping_add(1);
        }
    }

    pub fn set_viewport_height(&mut self, height: u16) {
        self.max_rows = (height as f32 * self.max_height.ratio()).round() as u16;
    }

    pub fn placement(
        &self,
        source: &ImageSource,
        size: Option<ImageSize>,
        max_width: usize,
    ) -> Placement {
        match self.entries.get(source) {
            Some(ImageEntry::Decoded { key, size: pixels }) => Placement::Image {
                key: key.clone(),
                size: cell_size(*pixels, self.font_size, max_width, self.max_rows, size),
            },
            Some(ImageEntry::Failed) => Placement::Failed,
            Some(ImageEntry::Loading) | None => Placement::Loading,
        }
    }
}

fn cell_size(
    pixels: PixelSize,
    font_size: Size,
    max_width: usize,
    max_rows: u16,
    size: Option<ImageSize>,
) -> Size {
    let font_width = font_size.width as f32;
    let font_height = font_size.height as f32;
    let natural_cols = (pixels.width as f32 / font_width).max(1.0);
    let natural_rows = (pixels.height as f32 / font_height).max(1.0);

    // Obsidian sizes are logical pixels; the cell size is in device pixels.
    let display_scale = (font_width / LOGICAL_CELL_WIDTH).round().max(1.0);
    let max_cols = match size {
        Some(size) => (size.width as f32 * display_scale / font_width).min(max_width as f32),
        None => max_width as f32,
    }
    .max(1.0);
    let max_rows = match size.and_then(|size| size.height) {
        Some(height) => (height as f32 * display_scale / font_height).min(max_rows as f32),
        None => max_rows as f32,
    }
    .max(1.0);

    let scale = (max_cols / natural_cols).min(max_rows / natural_rows);
    Size::new(
        (natural_cols * scale).round().max(1.0) as u16,
        (natural_rows * scale).round().max(1.0) as u16,
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImagePlacement {
    pub doc_line: usize,
    pub mark: ImageMark,
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

macro_rules! wrap_marker_span {
    ($span:expr) => {{
        VirtualSpan::WrapMarker($span.clone().into())
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
pub(crate) use wrap_marker_span;

#[derive(Clone, PartialEq, Debug)]
pub enum VirtualSpan<'a> {
    Synthetic(Span<'a>),
    WrapMarker(Span<'a>),
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
            Self::Synthetic(..) | Self::WrapMarker(..) => "".chars(),
        }
    }

    pub fn char_indices(&self) -> CharIndices<'_> {
        match self {
            Self::Content(span, ..) => span.content.char_indices(),
            Self::Synthetic(..) | Self::WrapMarker(..) => "".char_indices(),
        }
    }

    pub fn source_range(&self) -> Option<&SourceRange<usize>> {
        match self {
            Self::Content(.., source_range) => Some(source_range),
            Self::Synthetic(..) | Self::WrapMarker(..) => None,
        }
    }

    pub fn width(&self) -> usize {
        let span = match self {
            VirtualSpan::Content(span, ..)
            | VirtualSpan::Synthetic(span)
            | VirtualSpan::WrapMarker(span) => span,
        };
        // A tab is one byte but rendered as two columns (expanded at draw time).
        span.content
            .chars()
            .map(|c| if c == '\t' { 2 } else { c.width().unwrap_or(0) })
            .sum()
    }

    pub fn is_synthetic(&self) -> bool {
        matches!(
            self,
            VirtualSpan::Synthetic(..) | VirtualSpan::WrapMarker(..)
        )
    }

    pub fn is_wrap_marker(&self) -> bool {
        matches!(self, VirtualSpan::WrapMarker(..))
    }
}

impl<'a> From<VirtualSpan<'a>> for Span<'a> {
    fn from(value: VirtualSpan<'a>) -> Self {
        let span = match value {
            VirtualSpan::Synthetic(span)
            | VirtualSpan::WrapMarker(span)
            | VirtualSpan::Content(span, _) => span,
        };
        // Expand tabs so the terminal doesn't break the layout; the cursor maps
        // by byte offset against the un-expanded content (tabs counted as two).
        match span.content.contains('\t') {
            true => Span::styled(span.content.replace('\t', "  "), span.style),
            false => span,
        }
    }
}

/// Marks an image's first row, so placements are found at any nesting depth.
#[derive(Clone, PartialEq, Debug)]
pub struct ImageMark {
    pub key: ImageKey,
    pub size: Size,
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

    pub fn is_wrap_continuation(&self) -> bool {
        self.spans.iter().any(VirtualSpan::is_wrap_marker)
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
}

fn hash_str(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

type CodeTokens = Vec<highlight::LineTokens>;

fn code_cache_key(language: &str, text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    language.hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}

/// Highlighted token ranges for fenced code blocks, reused across a layout
/// pass. The key is a hash of the block's language and text, so a block with
/// unchanged text skips the syntax grammar entirely. A hash collision would
/// serve another block's tokens, which is accepted at 64 bits. Colours come
/// from tokens at render time, so a theme change does not invalidate this
/// cache.
///
/// Each pass takes the previous generation from `VirtualDocument::code_cache`
/// and hands back the next one. A block not read during the pass is dropped:
/// for example a deleted or renamed block, or one simply not visited. This
/// lets the cache clean itself with no size cap.
#[derive(Debug, Default)]
pub struct CodeCache {
    previous: HashMap<u64, CodeTokens>,
    next: HashMap<u64, CodeTokens>,
}

impl CodeCache {
    pub fn new(previous: HashMap<u64, CodeTokens>) -> Self {
        Self {
            previous,
            next: HashMap::new(),
        }
    }

    /// Token ranges for one code block. `None` when no grammar matches
    /// `language`. A hit moves the tokens from the previous generation into
    /// this one, so an unchanged block costs a map lookup and no copy.
    pub fn get_or_compute(&mut self, language: &str, text: &str) -> Option<&CodeTokens> {
        let key = code_cache_key(language, text);
        let tokens = match self.next.entry(key) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let tokens = match self.previous.remove(&key) {
                    Some(tokens) => tokens,
                    None => highlight::block_tokens(language, text)?,
                };
                entry.insert(tokens)
            }
        };
        Some(tokens)
    }

    /// The entries read during this pass, which seed the next one.
    pub fn into_next(self) -> HashMap<u64, CodeTokens> {
        self.next
    }
}

/// Fingerprint of every input that shapes a layout, so a frame whose inputs are
/// unchanged reuses the previous result instead of rebuilding the whole document.
/// The theme is not part of the key; `set_theme` clears the cache instead.
#[derive(Clone, Debug, PartialEq)]
struct LayoutKey {
    name_hash: u64,
    content_hash: u64,
    edit: bool,
    editing_block: Option<usize>,
    cursor_offset: usize,
    ast_len: usize,
    ast_last_end: usize,
    width: usize,
    horizontal_offset: usize,
    buffer: Option<(u64, SourceRange<usize>, bool)>,
    grammars_preloaded: bool,
    image_generation: u64,
    image_max_rows: u16,
}

#[derive(Clone, Debug, Default)]
pub struct VirtualDocument<'a> {
    symbols: Symbols,
    theme: Theme,
    meta: Vec<VirtualLine<'a>>,
    block_ranges: Vec<SourceRange<usize>>,
    lines: Vec<VirtualLine<'a>>,
    line_to_block: Vec<usize>,
    cache_key: Option<LayoutKey>,
    code_cache: HashMap<u64, CodeTokens>,
    image_placements: Vec<ImagePlacement>,
}

impl<'a> VirtualDocument<'a> {
    pub fn new(symbols: &Symbols) -> Self {
        Self {
            symbols: symbols.clone(),
            ..Default::default()
        }
    }

    pub fn set_theme(&mut self, theme: &Theme) {
        if self.theme != *theme {
            self.theme = *theme;
            self.cache_key = None;
        }
    }

    pub fn set_wrap(&mut self, wrap: bool) {
        self.symbols.wrap = wrap;
    }
    pub fn meta(&self) -> &[VirtualLine<'_>] {
        &self.meta
    }

    pub fn height(&self) -> usize {
        self.meta.len() + self.lines.len()
    }

    pub fn block_ranges(&self) -> &[SourceRange<usize>] {
        &self.block_ranges
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
        let edit = matches!(view, View::Edit(..));
        let key = LayoutKey {
            name_hash: hash_str(note_name),
            content_hash: hash_str(content),
            edit,
            editing_block: current_block_idx,
            // The cursor only reshapes layout while editing (it reveals its own
            // line raw in the active block); in Read mode it never does.
            cursor_offset: if edit { cursor_offset } else { 0 },
            ast_len: ast_nodes.len(),
            ast_last_end: ast_nodes.last().map_or(0, |node| node.source_range().end),
            width,
            horizontal_offset,
            buffer: text_buffer.as_ref().map(|buffer| {
                (
                    hash_str(&buffer.content),
                    buffer.source_range.clone(),
                    buffer.modified,
                )
            }),
            grammars_preloaded: highlight::is_preloaded(),
            image_generation: images.generation,
            image_max_rows: images.max_rows,
        };
        if self.cache_key.as_ref() == Some(&key) {
            return;
        }
        self.cache_key = Some(key);
        let mut code_cache = CodeCache::new(std::mem::take(&mut self.code_cache));

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
        let mut context = RenderContext {
            content: &live_content,
            max_width: width,
            horizontal_offset,
            option: &styled,
            symbols: &self.symbols,
            theme: &self.theme,
            code_cache: &mut code_cache,
            images,
        };

        let (block_ranges, lines, line_to_block) = ast_nodes.iter().enumerate().fold(
            (vec![], vec![], vec![]),
            |(mut block_ranges, mut lines, mut line_to_block), (idx, node)| {
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
                                &self.theme,
                            )
                        } else {
                            edit_lines(
                                buffer_content,
                                range.start,
                                cursor_offset,
                                width,
                                horizontal_offset,
                                &self.symbols,
                                &self.theme,
                            )
                        };
                        VirtualBlock::new(&lines, range)
                    }
                    None => render_node(node, Span::default(), 0, &mut context),
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
                    // the next block.
                    let trailing = match ast_nodes.get(idx + 1) {
                        None if is_active => 1,
                        None => {
                            let tail = live_content.get(end..).unwrap_or("");
                            let tail_blanks = tail.bytes().filter(|byte| *byte == b'\n').count();
                            let terminator = !slice.ends_with('\n') as usize;
                            let eof_line = live_content.ends_with('\n') as usize;
                            (tail_blanks.saturating_sub(terminator) + eof_line).max(1)
                        }
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

                    let final_line = ast_nodes.get(idx + 1).is_none()
                        && trailing > 0
                        && live_content.ends_with('\n');
                    let synthetic = trailing - final_line as usize;
                    block
                        .lines
                        .extend((0..synthetic).map(|_| empty_virtual_line!()));
                    if final_line {
                        let offset = live_content.len();
                        block.lines.push(virtual_line!([content_span!(
                            "".to_string(),
                            offset..offset
                        )]));
                    }
                }

                line_to_block.extend(iter::repeat_n(idx, block.lines.len()));
                lines.append(&mut block.lines);
                block_ranges.push(block.source_range);

                (block_ranges, lines, line_to_block)
            },
        );

        self.image_placements = lines
            .iter()
            .enumerate()
            .filter_map(|(doc_line, line)| {
                line.image_mark()
                    .cloned()
                    .map(|mark| ImagePlacement { doc_line, mark })
            })
            .collect();

        self.block_ranges = block_ranges;
        self.lines = lines;
        self.line_to_block = line_to_block;
        self.code_cache = code_cache.into_next();
    }
}

#[cfg(test)]
mod code_cache_tests {
    use super::*;

    #[test]
    fn reuses_cached_ranges_for_unchanged_text() {
        highlight::preload();
        let mut cache = CodeCache::default();
        let first = cache.get_or_compute("js", "const x = 1;").unwrap().clone();
        let mut cache = CodeCache::new(cache.into_next());
        let second = cache.get_or_compute("js", "const x = 1;").unwrap();
        assert_eq!(&first, second);
    }

    #[test]
    fn a_block_not_read_in_a_pass_is_dropped() {
        highlight::preload();
        let mut cache = CodeCache::default();
        cache.get_or_compute("js", "const x = 1;");
        let store = cache.into_next();
        assert_eq!(store.len(), 1);

        // A pass that reads a different block never touches the old key, so
        // it does not carry over to the next generation.
        let mut cache = CodeCache::new(store);
        cache.get_or_compute("js", "const y = 2;");
        assert_eq!(cache.into_next().len(), 1);
    }

    #[test]
    fn a_block_repeated_in_one_pass_keeps_one_entry() {
        highlight::preload();
        let mut cache = CodeCache::default();
        cache.get_or_compute("js", "const x = 1;");
        cache.get_or_compute("js", "const x = 1;");
        assert_eq!(cache.into_next().len(), 1);
    }

    #[test]
    fn unknown_language_returns_none() {
        assert!(CodeCache::default()
            .get_or_compute("no-such-language", "x")
            .is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 800x400px image, 10x20px font cells: 80x20 natural cells, 2:1 aspect.
    const PIXELS: PixelSize = PixelSize {
        width: 800,
        height: 400,
    };
    const FONT: Size = Size::new(10, 20);

    #[test]
    fn unsized_image_fills_the_width() {
        assert_eq!(cell_size(PIXELS, FONT, 40, 30, None), Size::new(40, 10));
    }

    #[test]
    fn explicit_width_caps_the_box_and_preserves_aspect() {
        let size = Some(ImageSize {
            width: 200,
            height: None,
        });
        assert_eq!(cell_size(PIXELS, FONT, 40, 30, size), Size::new(20, 5));
    }

    #[test]
    fn explicit_width_scales_up_on_a_hidpi_cell() {
        let size = Some(ImageSize {
            width: 200,
            height: None,
        });
        assert_eq!(cell_size(PIXELS, Size::new(16, 32), 80, 30, size).width, 25);
    }

    #[test]
    fn explicit_width_never_exceeds_the_available_width() {
        let size = Some(ImageSize {
            width: 9999,
            height: None,
        });
        assert_eq!(cell_size(PIXELS, FONT, 40, 30, size).width, 40);
    }

    #[test]
    fn explicit_height_bounds_the_box() {
        let size = Some(ImageSize {
            width: 9999,
            height: Some(100),
        });
        assert_eq!(cell_size(PIXELS, FONT, 40, 30, size), Size::new(20, 5));
    }
}
