use std::ops::{Deref, DerefMut};

use pulldown_cmark::{CodeBlockKind, Event, Options, Tag, TagEnd};

use crate::note_editor::{
    ast::{self, ImageSize, ImageSource, Node, SourceRange, TaskKind},
    rich_text::{RichText, Style, TextSegment},
};

pub struct Parser<'a> {
    events: pulldown_cmark::TextMergeWithOffset<'a, pulldown_cmark::OffsetIter<'a>>,
    source: &'a str,
}

impl<'a> Deref for Parser<'a> {
    type Target = pulldown_cmark::TextMergeWithOffset<'a, pulldown_cmark::OffsetIter<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl DerefMut for Parser<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.events
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = (Event<'a>, SourceRange<usize>);
    fn next(&mut self) -> Option<Self::Item> {
        self.deref_mut().next()
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ParserState {
    task_kind: Vec<ast::TaskKind>,
    item_kind: Vec<ast::ItemKind>,
}

impl<'a> Parser<'a> {
    /// Creates a new [`Parser`] from a Markdown input string.
    ///
    /// The parser uses [`pulldown_cmark::Parser::new_ext`] with [`Options::all()`] and
    /// [`pulldown_cmark::TextMergeWithOffset`] internally.
    ///
    /// The offset is required to know where the node appears in the provided source text.
    pub fn new(text: &'a str) -> Self {
        let mut options = Options::all();

        // Smart punctuation is excluded because it converts ASCII characters (e.g. ", ') to
        // multi-byte Unicode (“, ‘), making the rendered text longer than the source, causing the
        // source offset to overlap in some cases and causing unexpected behavior.
        //
        // TODO: Holistic approach to support smart punctation. Potentially need to do this on a
        // different layer of the app to only do the smart punctuation effect visually using
        // virtual elements or such, but keeping the original source content unchanged.
        options.remove(Options::ENABLE_SMART_PUNCTUATION);

        let parser = pulldown_cmark::TextMergeWithOffset::new(
            pulldown_cmark::Parser::new_ext(text, options).into_offset_iter(),
        );

        Self {
            events: parser,
            source: text,
        }
    }

    /// A thematic break (`---`, or a table's leftover dash row once the pipes are gone) is kept as
    /// a plain paragraph so its source stays visible and editable. Without this it parses as a
    /// [`pulldown_cmark::Event::Rule`], which carries no text and would leave that line uncovered
    /// by any node — invisible and impossible to fix. Horizontal rules are not rendered specially.
    fn rule_node(&self, source_range: SourceRange<usize>) -> Node {
        let text = self
            .source
            .get(source_range.clone())
            .unwrap_or("")
            .trim_end_matches('\n');
        Node::Paragraph {
            text: RichText::from(vec![TextSegment::plain(text)]),
            source_range,
        }
    }

    pub fn parse(mut self) -> Vec<Node> {
        let mut result = Vec::new();
        let mut state = ParserState::default();

        while let Some((event, source_range)) = self.next() {
            match event {
                Event::Start(Tag::Table(alignments)) => {
                    result.push(self.parse_table(alignments, source_range));
                }
                Event::Start(tag) if Self::is_container_tag(&tag) => {
                    if let Some(node) = self.parse_container(tag, &mut state) {
                        result.push(node);
                    }
                }
                Event::Rule => result.push(self.rule_node(source_range)),
                _ => {}
            }
        }

        result
    }

    pub fn parse_container(&mut self, tag: Tag, state: &mut ParserState) -> Option<Node> {
        let mut nodes = Vec::new();
        let mut text_segments = Vec::new();
        let mut inline_styles = Vec::new();

        match tag {
            Tag::List(Some(start)) => {
                state.item_kind.push(ast::ItemKind::Ordered(start));
            }
            Tag::List(..) => {
                state.item_kind.push(ast::ItemKind::Unordered);
            }
            _ => {}
        };

        while let Some((event, source_range)) = self.next() {
            match event {
                Event::Start(Tag::Table(alignments)) => {
                    nodes.push(self.parse_table(alignments, source_range));
                }

                Event::Start(inner_tag) if Self::is_container_tag(&inner_tag) => {
                    if let Some(node) = self.parse_container(inner_tag, state) {
                        nodes.push(node);
                    }
                }

                Event::Start(inner_tag) if Self::is_inline_tag(&inner_tag) => {
                    if let Some(style) = Self::tag_to_style(&inner_tag) {
                        inline_styles.push(style);
                    }
                }

                Event::Rule => nodes.push(self.rule_node(source_range)),

                Event::TaskListMarker(checked) => {
                    state.task_kind.push(if checked {
                        TaskKind::Checked
                    } else {
                        TaskKind::Unchecked
                    });
                }

                Event::Code(text) => {
                    let text_segment = TextSegment::styled(&text, Style::Code);
                    text_segments.push(text_segment);
                }

                Event::Text(text) => {
                    let mut text_segment = TextSegment::plain(&text);
                    inline_styles.iter().for_each(|style| {
                        text_segment.add_style(style);
                    });
                    text_segments.push(text_segment);
                }

                Event::SoftBreak => {
                    let text_segment = TextSegment::empty_line();
                    text_segments.push(text_segment);
                }

                Event::End(tag_end) if Self::tags_match(&tag, &tag_end) => {
                    let text = if !text_segments.is_empty() {
                        RichText::from(text_segments)
                    } else {
                        RichText::empty()
                    };

                    return match tag {
                        Tag::Heading { level, .. } => Some(Node::Heading {
                            level: level.into(),
                            text,
                            source_range,
                        }),
                        Tag::Item => {
                            // This is required since in block quotes list items are considered
                            // "tight", thus the text is not stored in a paragraph directly.
                            // TODO: Think if wrapping this into a paragraph is a good idea or not.
                            // Potentially storing a RichText here is better.
                            if !text.is_empty() {
                                nodes.insert(
                                    0,
                                    Node::Paragraph {
                                        text,
                                        source_range: source_range.clone(),
                                    },
                                );
                            }

                            let item = if let Some(kind) = state.task_kind.pop() {
                                Some(Node::Task {
                                    kind,
                                    nodes,
                                    source_range,
                                })
                            } else {
                                Some(Node::Item {
                                    kind: state
                                        .item_kind
                                        .last()
                                        .cloned()
                                        .unwrap_or(ast::ItemKind::Unordered),
                                    nodes,
                                    source_range,
                                })
                            };

                            if let Some(ast::ItemKind::Ordered(start)) = state.item_kind.last_mut()
                            {
                                *start += 1;
                            };

                            item
                        }
                        Tag::List(..) => {
                            state.item_kind.pop();

                            Some(Node::List {
                                nodes,
                                source_range,
                            })
                        }
                        Tag::CodeBlock(kind) => Some(Node::CodeBlock {
                            lang: match kind {
                                CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                                _ => None,
                            },
                            text,
                            source_range,
                        }),
                        Tag::BlockQuote(kind) => {
                            let (kind, title, nodes) =
                                resolve_callout(kind.map(|kind| kind.into()), nodes);
                            Some(Node::BlockQuote {
                                kind,
                                title,
                                nodes,
                                source_range,
                            })
                        }
                        Tag::Paragraph => Some(Node::Paragraph { text, source_range }),
                        _ => None,
                    };
                }
                _ => {}
            }
        }

        None
    }

    /// Parses a table from the events between `Start(Table)` and `End(Table)`.
    ///
    /// Cells before the first `TableRow` belong to the header; the rest form body rows. Inline
    /// styles are tracked per cell so emphasis inside a cell is preserved.
    fn parse_table(
        &mut self,
        alignments: Vec<pulldown_cmark::Alignment>,
        source_range: SourceRange<usize>,
    ) -> Node {
        let alignments = alignments.into_iter().map(Into::into).collect();
        let mut head = Vec::new();
        let mut rows = Vec::new();
        let mut row = Vec::new();
        let mut cell = Vec::new();
        let mut inline_styles = Vec::new();
        let mut in_head = false;

        for (event, _) in self.by_ref() {
            match event {
                Event::Start(Tag::TableHead) => in_head = true,
                Event::Start(Tag::TableRow) => in_head = false,
                Event::Start(Tag::TableCell) => {
                    cell = Vec::new();
                    inline_styles.clear();
                }
                Event::Start(inner_tag) if Self::is_inline_tag(&inner_tag) => {
                    if let Some(style) = Self::tag_to_style(&inner_tag) {
                        inline_styles.push(style);
                    }
                }
                Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough) => {
                    inline_styles.pop();
                }
                Event::Code(text) => cell.push(TextSegment::styled(&text, Style::Code)),
                Event::Text(text) => {
                    let mut text_segment = TextSegment::plain(&text);
                    inline_styles
                        .iter()
                        .for_each(|style| text_segment.add_style(style));
                    cell.push(text_segment);
                }
                Event::End(TagEnd::TableCell) => {
                    let text = RichText::from(std::mem::take(&mut cell));
                    if in_head {
                        head.push(text);
                    } else {
                        row.push(text);
                    }
                }
                Event::End(TagEnd::TableRow) => rows.push(std::mem::take(&mut row)),
                Event::End(TagEnd::Table) => break,
                _ => {}
            }
        }

        Node::Table {
            alignments,
            head,
            rows,
            source_range,
        }
    }

    fn is_container_tag(tag: &Tag) -> bool {
        matches!(
            tag,
            Tag::Paragraph
                | Tag::Item
                | Tag::List(..)
                | Tag::BlockQuote(..)
                | Tag::CodeBlock(..)
                | Tag::Heading { .. }
        )
    }

    fn is_inline_tag(tag: &Tag) -> bool {
        matches!(tag, Tag::Emphasis | Tag::Strong | Tag::Strikethrough)
    }

    fn tags_match(start: &Tag, end: &TagEnd) -> bool {
        fn tag_to_end(tag: &Tag) -> Option<TagEnd> {
            match tag {
                Tag::Heading { level, .. } => Some(TagEnd::Heading(*level)),
                Tag::List(ordered) => Some(TagEnd::List(ordered.is_some())),
                Tag::Item => Some(TagEnd::Item),
                Tag::BlockQuote(kind) => Some(TagEnd::BlockQuote(*kind)),
                Tag::CodeBlock(..) => Some(TagEnd::CodeBlock),
                Tag::Paragraph => Some(TagEnd::Paragraph),
                _ => None,
            }
        }

        if let Some(start) = tag_to_end(start) {
            std::mem::discriminant(&start) == std::mem::discriminant(end)
        } else {
            false
        }
    }

    fn tag_to_style(tag: &Tag) -> Option<Style> {
        match tag {
            Tag::Emphasis => Some(Style::Emphasis),
            Tag::Strong => Some(Style::Strong),
            Tag::Strikethrough => Some(Style::Strikethrough),
            _ => None,
        }
    }
}

/// Resolves a quote's callout kind and title. `pulldown_cmark` handles bare
/// GitHub alerts (`kind` is `Some`); otherwise we look for an Obsidian-style
/// marker on the first line and strip that line from the body.
fn resolve_callout(
    kind: Option<ast::BlockQuoteKind>,
    mut nodes: Vec<Node>,
) -> (Option<ast::BlockQuoteKind>, Option<String>, Vec<Node>) {
    if kind.is_some() {
        return (kind, None, nodes);
    }

    let stripped = match nodes.first() {
        Some(Node::Paragraph { text, source_range }) => {
            let segments = text.segments();
            let break_index = segments.iter().position(|segment| segment.content == "\n");
            let first_line: String = segments[..break_index.unwrap_or(segments.len())]
                .iter()
                .map(|segment| segment.content.as_str())
                .collect();
            ast::parse_callout_marker(&first_line).map(|marker| {
                let body = break_index
                    .map(|index| segments[index + 1..].to_vec())
                    .unwrap_or_default();
                (marker, body, source_range.clone())
            })
        }
        _ => None,
    };

    let Some((marker, body, source_range)) = stripped else {
        return (None, None, nodes);
    };

    if body.is_empty() {
        nodes.remove(0);
    } else {
        nodes[0] = Node::Paragraph {
            text: RichText::from(body),
            source_range,
        };
    }

    (Some(marker.kind), marker.title, nodes)
}

pub fn from_str(text: &str) -> Vec<Node> {
    normalize_images(Parser::new(text).parse(), text)
}

/// Rewrites any top-level paragraph whose every line is a block image into
/// [`Node::Image`]s, one per line. Covers standard `![alt](url)` images and
/// Obsidian `![[name]]` embeds (which pulldown-cmark leaves as plain text).
/// Consecutive embed lines form a single CommonMark paragraph, so each line is
/// split out. A paragraph that mixes images with prose is left untouched.
fn normalize_images(nodes: Vec<Node>, source: &str) -> Vec<Node> {
    // The start of each node, used to clamp a paragraph's split range so it does
    // not reach into a following sibling. In a loose list item the leading
    // paragraph's source range spans the whole item (including a nested list),
    // which would otherwise double-count the nested embeds.
    let next_starts: Vec<Option<usize>> = (0..nodes.len())
        .map(|i| nodes.get(i + 1).map(|node| node.source_range().start))
        .collect();

    nodes
        .into_iter()
        .enumerate()
        .flat_map(|(i, mut node)| {
            // Recurse into containers (lists, items, block quotes) so embeds
            // nested under list items are recognized too.
            if let Some(children) = node.children_as_mut() {
                let normalized = normalize_images(std::mem::take(children), source);
                *children = normalized;
                return vec![node];
            }
            match &node {
                Node::Paragraph { source_range, .. } => {
                    let end = next_starts[i]
                        .filter(|&start| (source_range.start..source_range.end).contains(&start))
                        .unwrap_or(source_range.end);
                    source
                        .get(source_range.start..end)
                        .and_then(|slice| split_block_images(slice, source_range.start))
                        .unwrap_or_else(|| vec![node])
                }
                _ => vec![node],
            }
        })
        .collect()
}

/// Splits a paragraph's source into image nodes for the lines that are block
/// images, with the runs of prose between them kept as paragraph nodes. Returns
/// `None` when the paragraph holds no image at all, so it is left untouched.
fn split_block_images(slice: &str, start: usize) -> Option<Vec<Node>> {
    let is_image = |line: &str| {
        let trimmed = line.trim();
        (!trimmed.is_empty())
            .then(|| parse_block_image(trimmed))
            .flatten()
    };

    if !slice
        .split_inclusive('\n')
        .any(|line| is_image(line).is_some())
    {
        return None;
    }

    let mut nodes = Vec::new();
    let mut offset = start;
    let mut prose_start = start;
    let mut prose = String::new();

    let flush_prose = |nodes: &mut Vec<Node>, prose: &mut String, prose_start: usize| {
        if !prose.trim().is_empty() {
            let mut parsed = Parser::new(prose).parse();
            parsed
                .iter_mut()
                .for_each(|node| offset_source_range(node, prose_start));
            nodes.append(&mut parsed);
        }
        prose.clear();
    };

    for line in slice.split_inclusive('\n') {
        match is_image(line) {
            Some((source, alt, size)) => {
                flush_prose(&mut nodes, &mut prose, prose_start);
                nodes.push(Node::Image {
                    source,
                    alt,
                    size,
                    source_range: offset..offset + line.len(),
                });
                prose_start = offset + line.len();
            }
            None => prose.push_str(line),
        }
        offset += line.len();
    }
    flush_prose(&mut nodes, &mut prose, prose_start);

    Some(nodes)
}

/// Shifts a node's source range (and its children's) by `by` bytes, used when
/// re-parsing a prose run that started partway into the source.
fn offset_source_range(node: &mut Node, by: usize) {
    let range = node.source_range();
    node.set_source_range(range.start + by..range.end + by);
    if let Some(children) = node.children_as_mut() {
        children
            .iter_mut()
            .for_each(|child| offset_source_range(child, by));
    }
}

/// Parses a line that is exactly one image into its source, alt text and any
/// explicit pixel size (`|width` / `|widthxheight`).
fn parse_block_image(line: &str) -> Option<(ImageSource, String, Option<ImageSize>)> {
    if line.contains('\n') {
        return None;
    }

    // Obsidian embed: `![[target#subpath|size]]` — resolve by file name.
    if let Some(inner) = line.strip_prefix("![[").and_then(|s| s.strip_suffix("]]")) {
        if inner.contains("]]") || inner.is_empty() {
            return None;
        }
        let target = inner.split(['|', '#']).next().unwrap_or(inner).trim();
        let size = inner
            .rsplit_once('|')
            .and_then(|(_, spec)| parse_image_size(spec.trim()));
        return Some((
            ImageSource::Embed(target.to_string()),
            target.to_string(),
            size,
        ));
    }

    // Standard markdown image: `![alt|size](dest "title")`.
    let rest = line.strip_prefix("![")?.strip_suffix(')')?;
    let close = rest.find("](")?;
    let alt = &rest[..close];
    if alt.contains("](") {
        return None;
    }
    let dest = rest[close + 2..].split_whitespace().next()?;
    if dest.is_empty() {
        return None;
    }

    let source = if dest.starts_with("http://") || dest.starts_with("https://") {
        ImageSource::Url(dest.to_string())
    } else {
        ImageSource::Path(dest.to_string())
    };
    let (alt, size) = split_image_size(alt);
    Some((source, alt.to_string(), size))
}

/// Splits a trailing `|size` off the alt text. When the segment after the last
/// `|` is not a valid pixel size it is left in place and no size is returned.
fn split_image_size(alt: &str) -> (&str, Option<ImageSize>) {
    match alt.rsplit_once('|') {
        Some((label, spec)) => match parse_image_size(spec.trim()) {
            Some(size) => (label.trim_end(), Some(size)),
            None => (alt, None),
        },
        None => (alt, None),
    }
}

/// Parses an Obsidian pixel-size spec: `200` -> width only, `200x300` -> both.
fn parse_image_size(spec: &str) -> Option<ImageSize> {
    let (width, height) = match spec.split_once(['x', 'X']) {
        Some((width, height)) => (width, Some(height)),
        None => (spec, None),
    };
    Some(ImageSize {
        width: width.trim().parse().ok()?,
        height: match height {
            Some(height) => Some(height.trim().parse().ok()?),
            None => None,
        },
    })
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn test_parser() {
        let tests = [
            (
                "paragraphs",
                indoc! { r#"## Paragraphs
                To create paragraphs in Markdown, use a **blank line** to separate blocks of text. Each block of text separated by a blank line is treated as a distinct paragraph.

                This is a paragraph.

                This is another paragraph.

                A blank line between lines of text creates separate paragraphs. This is the default behavior in Markdown.
                "#},
            ),
            (
                "headings",
                indoc! { r#"## Headings
                To create a heading, add up to six `#` symbols before your heading text. The number of `#` symbols determines the size of the heading.

                # This is a heading 1
                ## This is a heading 2
                ### This is a heading 3
                #### This is a heading 4
                ##### This is a heading 5
                ###### This is a heading 6
                "#},
            ),
            (
                "lists",
                indoc! { r#"## Lists
                You can create an unordered list by adding a `-`, `*`, or `+` before the text.

                - First list item
                - Second list item
                - Third list item

                To create an ordered list, start each line with a number followed by a `.` or `)` symbol.

                1. First list item
                2. Second list item
                3. Third list item

                1) First list item
                2) Second list item
                3) Third list item
                "#},
            ),
            (
                "lists_line_breaks",
                indoc! { r#"## Lists with line breaks
                You can use line breaks within an ordered list without altering the numbering.

                1. First list item

                2. Second list item
                3. Third list item

                4. Fourth list item
                5. Fifth list item
                6. Sixth list item
                "#},
            ),
            (
                "task_lists",
                indoc! { r#"## Task lists
                To create a task list, start each list item with a hyphen and space followed by `[ ]`.

                - [x] This is a completed task.
                - [ ] This is an incomplete task.

                You can toggle a task in Reading view by selecting the checkbox.

                > [!tip]
                > You can use any character inside the brackets to mark it as complete.
                >
                > - [x] Milk
                > - [?] Eggs
                > - [-] Eggs
                "#},
            ),
            (
                "callouts",
                indoc! { r#"## Callouts

                > [!NOTE]
                > Strict GitHub form.

                > [!summary] Aliased to abstract
                > Body line.

                > [!danger]- Foldable with a title
                > Body line.

                > [!custom] Unknown defaults to note
                > Body line.

                > A plain quote, not a callout.
                "#},
            ),
            (
                "nesting_lists",
                indoc! { r#"## Nesting lists
                You can nest any type of list—ordered, unordered, or task lists—under any other type of list.

                To create a nested list, indent one or more list items. You can mix list types within a nested structure:

                1. First list item
                   1. Ordered nested list item
                2. Second list item
                   - Unordered nested list item
                "#},
            ),
            (
                "nesting_task_lists",
                indoc! { r#"## Nesting task lists
                Similarly, you can create a nested task list by indenting one or more list items:

                - [ ] Task item 1
                  - [ ] Subtask 1
                - [ ] Task item 2
                  - [ ] Subtask 2
                "#},
            ),
            (
                // A thematic break (and a table's leftover dash row once the pipes are
                // deleted) is kept as a plain paragraph so its source stays visible and
                // editable rather than vanishing.
                "horizontal_rule",
                indoc! { r#"## Horizontal rule

                ---

                A broken table degrades to a dash row, which must stay visible:

                First Header | Second Header
                ------------ ------------
                Content | Content
                "#},
            ),
            (
                "code_blocks",
                indoc! { r#"## Code blocks
                To format code as a block, enclose it with three backticks or three tildes.

                ```md
                cd ~/Desktop
                ```

                You can also create a code block by indenting the text using `Tab` or 4 blank spaces.

                    cd ~/Desktop

                "#},
            ),
            (
                "tables",
                indoc! { r#"## Tables
                You can create a table by separating columns with `|` and the header from the body with a row of dashes.

                | Name  | Role      | Notes                       |
                | :---- | :-------: | --------------------------: |
                | Alice | Maintainer | Writes most of the **core** code |
                | Bob   | Reviewer  | `reviews`                   |
                "#},
            ),
            (
                "code_syntax_highlighting_in_blocks",
                indoc! { r#"## Code syntax highlighting in blocks
                You can add syntax highlighting to a code block, by adding a language code after the first set of backticks.

                ```js
                function fancyAlert(arg) {
                  if(arg) {
                    $.facebox({div:'#foo'})
                  }
                }
                ```
                "#},
            ),
        ];

        tests.into_iter().for_each(|(name, text)| {
            assert_snapshot!(
                name,
                format!(
                    "{}\n ---\n\n{}",
                    text,
                    ast::nodes_to_sexp(&from_str(text), 0)
                )
            );
        });
    }

    #[test]
    fn test_image_parsing() {
        let text = indoc! { r#"## Images

            ![[diagram.png]]

            ![[photo.png|240]]

            ![[chart.png|200x300]]

            ![alt text](./local.png)

            ![Obsidian|200](https://obsidian.md/banner.png)

            ![remote](https://example.com/pic.png)

            An inline ![x](y.png) image stays a paragraph.
            "#};

        assert_snapshot!(format!(
            "{}\n ---\n\n{}",
            text,
            ast::nodes_to_sexp(&from_str(text), 0)
        ));
    }

    #[test]
    fn test_consecutive_embeds_split_into_images() {
        // Consecutive embed lines form one CommonMark paragraph; each line must
        // still become its own image node (as Obsidian renders them).
        let count = |text: &str| {
            from_str(text)
                .iter()
                .filter(|node| matches!(node, ast::Node::Image { .. }))
                .count()
        };

        assert_eq!(count("![[a.png]]\n"), 1);
        assert_eq!(count("![[a.png]]\n![[b.png]]\n![[c.png]]\n"), 3);
        assert_eq!(count("![one](a.png)\n![[b.png]]\n"), 2);
        // Embeds adjacent to prose (no blank line) are still extracted; the
        // prose lines around them stay as paragraphs.
        assert_eq!(count("Example:\n![[a.png]]\n"), 1);
        assert_eq!(count("![[a.png]]\n![[b.png]]\ntrailing text\n"), 2);
        // A paragraph with no image at all is untouched.
        assert_eq!(count("just text\nmore text\n"), 0);
    }
}
