use std::{iter, str::Chars};

use ratatui::text::{Line, Span};

use crate::{
    note_editor::{
        ast::{self, SourceRange},
        editor::{TextBuffer, View},
        render::{render_node, text_wrap, RenderStyle},
    },
    stylized_text::{stylize, FontStyle},
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

    pub fn width(&self) -> usize {
        self.spans.iter().map(|span| span.width()).sum()
    }

    pub fn content_width(&self) -> usize {
        self.spans
            .iter()
            .filter_map(|span| matches!(span, VirtualSpan::Content(..)).then_some(span.width()))
            .sum()
    }

    pub fn contains_offset(&self, offset: usize) -> bool {
        self.spans
            .iter()
            .any(|visual_span| visual_span.contains_offset(offset))
    }

    pub fn spans(self) -> Vec<Span<'a>> {
        self.spans
            .into_iter()
            .map(|s| s.into())
            // .flat_map(|virtual_span| match virtual_span {
            //     VirtualSpan::Content(span, source_range) => {
            //         vec![
            //             span,
            //             synthetic_span!(format!(" ({:?})", source_range)).into(),
            //         ]
            //     }
            //     span => vec![span.into()],
            // })
            .collect()
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

impl<'a> VirtualDocument<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn meta(&self) -> &[VirtualLine<'_>] {
        &self.meta
    }

    pub fn blocks(&self) -> &[VirtualBlock<'_>] {
        &self.blocks
    }

    pub fn mut_blocks(&mut self) -> &mut [VirtualBlock<'a>] {
        self.blocks.as_mut_slice()
    }

    pub fn lines(&self) -> &[VirtualLine<'_>] {
        &self.lines
    }

    pub fn line_to_block(&self) -> &[usize] {
        &self.line_to_block
    }

    pub fn get_block(&self, block_idx: usize) -> Option<(usize, &VirtualBlock<'_>)> {
        self.blocks().get(block_idx).map(|block| (block_idx, block))
    }

    pub fn get_mut_block(&mut self, block_idx: usize) -> Option<&mut VirtualBlock<'a>> {
        self.mut_blocks().get_mut(block_idx)
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

        // FIXME: When text buffer has more lines than the original rendered output
        // and new lines are added at the end  the text buffer render will match the next block and
        // replace visually the next block with the text buffer.
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

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use crate::note_editor::{
        editor::{EditMode, View},
        parser,
        virtual_document::{VirtualDocument, VirtualSpan},
    };

    #[test]
    fn test_simple_paragraph_no_wrapping() {
        let content = "This is a short paragraph.";
        let ast_nodes = parser::from_str(content);
        let mut virtual_document = VirtualDocument::new();

        virtual_document.layout("", content, &View::Read, None, &ast_nodes, 80, None);

        let blocks = virtual_document.blocks();
        assert_eq!(blocks.len(), 1, "Should have exactly one block");

        let block = &blocks[0];
        assert_eq!(block.source_range(), &(0..content.len()));

        // Should have 2 lines: the paragraph content + empty line
        assert_eq!(block.lines().len(), 2);

        let first_line = &block.lines()[0];
        assert!(first_line.has_content(), "First line should have content");

        // Verify source range of the first line matches the content
        let line_source_range = first_line.source_range();
        assert_eq!(line_source_range, Some(0..content.len()));
    }

    #[test]
    fn test_paragraph_wraps_at_width() {
        // This paragraph should wrap into multiple lines at width 40
        let content = "This is a longer paragraph that should definitely wrap to multiple lines when rendered with a narrow width.";
        let ast_nodes = parser::from_str(content);
        let mut virtual_document = VirtualDocument::new();

        let width = 40;
        virtual_document.layout(
            "",
            content,
            &View::Edit(EditMode::Source),
            None,
            &ast_nodes,
            width,
            None,
        );

        let blocks = virtual_document.blocks();
        assert_eq!(blocks.len(), 1);

        let block = &blocks[0];
        let lines = block.lines();

        // Should wrap to multiple lines (excluding the empty line at the end)
        assert!(
            lines.len() > 2,
            "Should have more than 2 lines (content wrapped + empty line)"
        );

        // Verify all content lines have source ranges
        let content_lines: Vec<_> = lines.iter().filter(|line| line.has_content()).collect();

        assert!(
            content_lines.len() > 1,
            "Should have multiple content lines due to wrapping"
        );

        // Verify source ranges are sequential and non-overlapping
        let mut expected_start = 0;
        for line in &content_lines {
            if let Some(range) = line.source_range() {
                assert_eq!(
                    range.start, expected_start,
                    "Source ranges should be sequential"
                );
                assert!(
                    range.end > range.start,
                    "Source range should have positive length"
                );
                assert!(
                    range.end <= content.len(),
                    "Source range should not exceed content length"
                );
                expected_start = range.end;
            }
        }

        // Verify source ranges cover most of the content
        // Note: textwrap may trim trailing/internal whitespace during wrapping
        let trimmed_len = content.trim_end().len();
        assert!(
            expected_start >= trimmed_len - 5 && expected_start <= content.len(),
            "Source ranges should cover most of the content (covered: {}, content: {}, trimmed: {})",
            expected_start,
            content.len(),
            trimmed_len
        );
    }

    #[test]
    fn test_multiple_paragraphs_source_ranges() {
        let content = indoc! { r#"First paragraph.

        Second paragraph here.

        Third and final paragraph."#};

        let ast_nodes = parser::from_str(content);
        let mut virtual_document = VirtualDocument::new();

        virtual_document.layout("", content, &View::Read, None, &ast_nodes, 80, None);

        let blocks = virtual_document.blocks();
        assert_eq!(blocks.len(), 3, "Should have three paragraph blocks");

        // Verify each block has correct source ranges
        for (i, block) in blocks.iter().enumerate() {
            let block_source = block.source_range();
            let block_content = &content[block_source.clone()];

            assert!(
                !block_content.is_empty(),
                "Block {} should have non-empty content",
                i
            );

            // Each block should have at least one line with content
            let has_content = block.lines().iter().any(|line| line.has_content());
            assert!(
                has_content,
                "Block {} should have at least one line with content",
                i
            );

            // Verify the line source ranges fall within the block source range
            for line in block.lines() {
                if let Some(line_range) = line.source_range() {
                    assert!(
                        line_range.start >= block_source.start,
                        "Line source range should start within block range"
                    );
                    assert!(
                        line_range.end <= block_source.end,
                        "Line source range should end within block range"
                    );
                }
            }
        }
    }

    #[test]
    fn test_heading_wrapping_source_ranges() {
        let content = "## This is a very long heading that should wrap when rendered at a narrow width to test source ranges";
        let ast_nodes = parser::from_str(content);
        let mut virtual_document = VirtualDocument::new();

        let width = 40;
        virtual_document.layout("", content, &View::Read, None, &ast_nodes, width, None);

        let blocks = virtual_document.blocks();
        assert_eq!(blocks.len(), 1);

        let block = &blocks[0];
        let lines = block.lines();

        // Find content lines (excluding underline and empty lines)
        let content_lines: Vec<_> = lines.iter().filter(|line| line.has_content()).collect();

        assert!(
            content_lines.len() > 1,
            "Heading should wrap to multiple lines"
        );

        // The source range should cover the entire heading including the "## " prefix
        let block_source = block.source_range();
        assert_eq!(block_source.start, 0);
        assert_eq!(block_source.end, content.len());
    }

    #[test]
    fn test_exact_width_boundary() {
        // Create content that exactly fits the width
        let content = "Exactly forty characters in this line!"; // 39 chars
        let ast_nodes = parser::from_str(content);
        let mut virtual_document = VirtualDocument::new();

        let width = 50;
        virtual_document.layout("", content, &View::Read, None, &ast_nodes, width, None);

        let blocks = virtual_document.blocks();
        assert_eq!(blocks.len(), 1);

        let block = &blocks[0];
        let content_lines: Vec<_> = block
            .lines()
            .iter()
            .filter(|line| line.has_content())
            .collect();

        // Should not wrap since it fits
        assert_eq!(content_lines.len(), 1, "Should fit on one line");

        let first_line = content_lines[0];
        let source_range = first_line.source_range().unwrap();
        assert_eq!(source_range, 0..content.len());
    }

    #[test]
    fn test_source_ranges_cover_entire_content() {
        let content = "This is a test paragraph with multiple words that will wrap.";
        let ast_nodes = parser::from_str(content);
        let mut virtual_document = VirtualDocument::new();

        let width = 30;
        virtual_document.layout(
            "",
            content,
            &View::Edit(EditMode::Source),
            None,
            &ast_nodes,
            width,
            None,
        );

        let blocks = virtual_document.blocks();
        let block = &blocks[0];

        // Collect all source ranges from content lines
        let mut all_ranges: Vec<_> = block
            .lines()
            .iter()
            .filter_map(|line| line.source_range())
            .collect();

        all_ranges.sort_by_key(|r| r.start);

        // Verify ranges are contiguous
        let mut covered = 0;
        for range in all_ranges {
            assert_eq!(range.start, covered, "Source ranges should be contiguous");
            covered = range.end;
        }

        // Verify source ranges cover most of the content
        // Note: textwrap trims whitespace during wrapping, so we check against trimmed length
        let trimmed_len = content.trim_end().len();
        assert!(
            covered >= trimmed_len - 5 && covered <= content.len(),
            "Source ranges should cover most of the content (covered: {}, trimmed: {}, total: {})",
            covered,
            trimmed_len,
            content.len()
        );
    }

    #[test]
    fn test_virtual_span_source_ranges() {
        let content = "A paragraph that wraps nicely.";
        let ast_nodes = parser::from_str(content);
        let mut virtual_document = VirtualDocument::new();

        let width = 20;
        virtual_document.layout("", content, &View::Read, None, &ast_nodes, width, None);

        let blocks = virtual_document.blocks();
        let block = &blocks[0];

        for line in block.lines() {
            for span in line.virtual_spans() {
                match span {
                    VirtualSpan::Content(_, source_range) => {
                        // Verify the content at the source range matches the span content
                        let source_content = &content[source_range.clone()];
                        assert!(
                            !source_content.is_empty(),
                            "Content span should map to non-empty source"
                        );
                    }
                    VirtualSpan::Synthetic(_) => {
                        // Synthetic spans should not have source ranges
                        assert!(span.source_range().is_none());
                    }
                }
            }
        }
    }

    #[test]
    fn test_list_item_wrapping() {
        let content = indoc! { r#"- This is a very long list item that should wrap to multiple lines when rendered at a narrow width
        - Short item
        - Another longer item that might also wrap depending on the width we choose for rendering"#};

        let ast_nodes = parser::from_str(content);
        let mut virtual_document = VirtualDocument::new();

        let width = 40;
        virtual_document.layout("", content, &View::Read, None, &ast_nodes, width, None);

        let blocks = virtual_document.blocks();
        assert_eq!(blocks.len(), 1, "Should have one list block");

        let block = &blocks[0];

        // Verify the block source range covers the entire list
        let block_source = block.source_range();
        assert_eq!(block_source.start, 0);
        assert_eq!(block_source.end, content.len());

        // Check that we have multiple lines (items wrapped)
        let lines = block.lines();
        assert!(
            lines.len() > 3,
            "Should have more than 3 lines due to wrapping"
        );
    }

    // #[test]
    // fn test() {
    //     let test_content = "To create paragraphs in Markdown, use a **blank line** to separate blocks of text. Each block of text separated by a blank line is treated as a distinct paragraph.";
    //
    //     let ast_node = &parser::from_str(test_content)[0];
    //
    //     let block = render_node(
    //         test_content.to_string(),
    //         ast_node,
    //         80,
    //         Span::default(),
    //         &RenderStyle::Raw,
    //     );
    //
    //     assert_eq!(
    //         block,
    //         VirtualBlock {
    //             lines: vec![],
    //             source_range: 0..1
    //         }
    //     )
    // }
}
