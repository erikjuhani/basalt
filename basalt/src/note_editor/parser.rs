use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;

use pulldown_cmark::{CodeBlockKind, Event, Options, Tag, TagEnd};

use crate::note_editor::{
    ast::{self, Node, SourceRange, TaskKind},
    rich_text::{RichText, Style, TextSegment, InlineNode},
};

pub struct Parser<'a>(pulldown_cmark::TextMergeWithOffset<'a, pulldown_cmark::OffsetIter<'a>>);

impl<'a> Deref for Parser<'a> {
    type Target = pulldown_cmark::TextMergeWithOffset<'a, pulldown_cmark::OffsetIter<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Parser<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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

        Self(parser)
    }

    pub fn parse(mut self) -> Vec<Node> {
        let mut result = Vec::new();
        let mut state = ParserState::default();
        let mut footnote_defs: IndexMap<String, RichText> = IndexMap::new();

        while let Some((event, _range)) = self.next() {
            match event {
                Event::Start(tag) if Self::is_container_tag(&tag) => {
                    if let Some(node) = self.parse_container(tag, &mut state, &mut footnote_defs) {
                        result.push(node);
                    }
                }
                _ => {}
            }
        }

        // Append FootnoteSection at document end if any definitions were collected
        if !footnote_defs.is_empty() {
            let last_range = result
                .last()
                .map(|n| n.source_range().clone())
                .unwrap_or(0..0);
            result.push(Node::FootnoteSection {
                defs: footnote_defs,
                source_range: last_range.end..last_range.end,
            });
        }

        result
    }

    pub fn parse_container(
        &mut self,
        tag: Tag,
        state: &mut ParserState,
        footnote_defs: &mut IndexMap<String, RichText>,
    ) -> Option<Node> {
        let mut nodes = Vec::new();
        let mut text_segments: Vec<InlineNode> = Vec::new();
        let mut inline_styles = Vec::new();
        let mut link_accumulator: Option<(String, Vec<String>)> = None; // http/https links only

        // Table parsing: dedicated accumulation loop that processes all table events
        // and returns immediately, bypassing the generic event loop below.
        //
        // pulldown-cmark 0.13 table event structure:
        //   Start(Table([alignments]))
        //     Start(TableHead)
        //       Start(TableCell) ... End(TableCell)  ← header cells (no TableRow wrapper!)
        //     End(TableHead)
        //     Start(TableRow)
        //       Start(TableCell) ... End(TableCell)
        //     End(TableRow)
        //     ...
        //   End(Table)
        if let Tag::Table(ref alignments_raw) = tag {
            let alignments: Vec<ast::Alignment> =
                alignments_raw.iter().copied().map(ast::Alignment::from).collect();
            let mut header: Vec<RichText> = Vec::new();
            let mut rows: Vec<Vec<RichText>> = Vec::new();
            // current_cells accumulates cells in header (during TableHead) or in a body row
            let mut current_cells: Vec<RichText> = Vec::new();
            let mut cell_segments: Vec<TextSegment> = Vec::new();
            let mut cell_styles: Vec<Style> = Vec::new();
            let mut source_range = 0..0;

            for (event, range) in self.by_ref() {
                match event {
                    Event::Start(Tag::TableHead) => {
                        current_cells = Vec::new();
                    }
                    Event::End(TagEnd::TableHead) => {
                        // Header cells are collected directly (no TableRow wrapper in TableHead)
                        header = std::mem::take(&mut current_cells);
                    }
                    Event::Start(Tag::TableRow) => {
                        current_cells = Vec::new();
                    }
                    Event::End(TagEnd::TableRow) => {
                        rows.push(std::mem::take(&mut current_cells));
                    }
                    Event::Start(Tag::TableCell) => {
                        cell_segments = Vec::new();
                        cell_styles = Vec::new();
                    }
                    Event::End(TagEnd::TableCell) => {
                        let rt = if cell_segments.is_empty() {
                            RichText::empty()
                        } else {
                            RichText::from(std::mem::take(&mut cell_segments))
                        };
                        current_cells.push(rt);
                    }
                    Event::Text(text) => {
                        let mut seg = TextSegment::plain(&text);
                        cell_styles.iter().for_each(|s| seg.add_style(s));
                        cell_segments.push(seg);
                    }
                    Event::Code(text) => {
                        cell_segments.push(TextSegment::styled(&text, Style::Code));
                    }
                    Event::Start(Tag::Emphasis) => {
                        cell_styles.push(Style::Emphasis);
                    }
                    Event::Start(Tag::Strong) => {
                        cell_styles.push(Style::Strong);
                    }
                    Event::Start(Tag::Strikethrough) => {
                        cell_styles.push(Style::Strikethrough);
                    }
                    Event::End(TagEnd::Emphasis) => {
                        cell_styles.retain(|s| !matches!(s, Style::Emphasis));
                    }
                    Event::End(TagEnd::Strong) => {
                        cell_styles.retain(|s| !matches!(s, Style::Strong));
                    }
                    Event::End(TagEnd::Strikethrough) => {
                        cell_styles.retain(|s| !matches!(s, Style::Strikethrough));
                    }
                    Event::End(TagEnd::Table) => {
                        source_range = range;
                        break;
                    }
                    _ => {}
                }
            }

            return Some(Node::Table {
                alignments,
                header,
                rows,
                source_range,
            });
        }

        // FootnoteDefinition: dedicated accumulation loop similar to tables
        if let Tag::FootnoteDefinition(ref label) = tag {
            let label = label.to_string();
            let mut def_segments: Vec<InlineNode> = Vec::new();
            let mut def_styles: Vec<Style> = Vec::new();

            for (event, _range) in self.by_ref() {
                match event {
                    Event::Text(text) => {
                        let mut seg = TextSegment::plain(&text);
                        def_styles.iter().for_each(|s| seg.add_style(s));
                        def_segments.push(InlineNode::Text(seg));
                    }
                    Event::Code(text) => {
                        def_segments.push(InlineNode::Text(TextSegment::styled(&text, Style::Code)));
                    }
                    Event::SoftBreak => {
                        def_segments.push(InlineNode::Text(TextSegment::empty_line()));
                    }
                    Event::Start(Tag::Emphasis) => def_styles.push(Style::Emphasis),
                    Event::Start(Tag::Strong) => def_styles.push(Style::Strong),
                    Event::Start(Tag::Strikethrough) => def_styles.push(Style::Strikethrough),
                    Event::End(TagEnd::Emphasis) => {
                        def_styles.retain(|s| !matches!(s, Style::Emphasis));
                    }
                    Event::End(TagEnd::Strong) => {
                        def_styles.retain(|s| !matches!(s, Style::Strong));
                    }
                    Event::End(TagEnd::Strikethrough) => {
                        def_styles.retain(|s| !matches!(s, Style::Strikethrough));
                    }
                    // Skip inner paragraph wrappers — definition content is inline
                    Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) => {}
                    Event::End(TagEnd::FootnoteDefinition) => break,
                    _ => {}
                }
            }

            let def_text = if def_segments.is_empty() {
                RichText::empty()
            } else {
                RichText::from(def_segments)
            };
            footnote_defs.insert(label, def_text);

            // FootnoteDefinition does NOT produce a top-level Node
            return None;
        }

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
                Event::Start(inner_tag) if Self::is_container_tag(&inner_tag) => {
                    if let Some(node) = self.parse_container(inner_tag, state, footnote_defs) {
                        nodes.push(node);
                    }
                }

                Event::Start(inner_tag) if Self::is_inline_tag(&inner_tag) => {
                    if link_accumulator.is_some() {
                        // Inside link: ignore inline styling tags for AST simplification
                        // Link text is rendered as plain, styled via rich_text_to_spans
                    } else if let Some(style) = Self::tag_to_style(&inner_tag) {
                        inline_styles.push(style);
                    }
                }

                Event::End(TagEnd::Emphasis) | Event::End(TagEnd::Strong) | Event::End(TagEnd::Strikethrough) => {
                    if link_accumulator.is_some() {
                        // Inside link: ignore end tags
                    } else {
                        inline_styles.retain(|s| !matches!(
                            s,
                            Style::Emphasis | Style::Strong | Style::Strikethrough
                        ));
                    }
                }

                Event::TaskListMarker(checked) => {
                    state.task_kind.push(if checked {
                        TaskKind::Checked
                    } else {
                        TaskKind::Unchecked
                    });
                }

                Event::Code(text) => {
                    // If inside a link accumulator, collect code text for the link
                    if let Some((_, ref mut text_pieces)) = link_accumulator {
                        text_pieces.push(text.to_string());
                    } else {
                        let text_segment = TextSegment::styled(&text, Style::Code);
                        text_segments.push(InlineNode::Text(text_segment));
                    }
                }

                Event::FootnoteReference(label) => {
                    // Inline footnote reference marker (e.g. [^1] -> FootnoteRef("1"))
                    text_segments.push(InlineNode::FootnoteRef(label.to_string()));
                }

                Event::Start(Tag::Link { dest_url, .. }) => {
                    // Only accumulate http/https links — other schemes render as plain text
                    if dest_url.starts_with("http://") || dest_url.starts_with("https://") {
                        link_accumulator = Some((dest_url.to_string(), Vec::new()));
                    }
                }

                Event::End(TagEnd::Link) => {
                    if let Some((url, text_pieces)) = link_accumulator.take() {
                        let combined_text = text_pieces.join("");
                        use crate::note_editor::rich_text::LinkTarget;
                        text_segments.push(InlineNode::Link {
                            text: combined_text,
                            target: LinkTarget::External(url),
                        });
                    }
                }

                Event::Text(text) => {
                    // If inside a link accumulator, collect text for the link
                    if let Some((_, ref mut text_pieces)) = link_accumulator {
                        text_pieces.push(text.to_string());
                    } else {
                        // ITS Theme task marker detection: only on the very first text segment
                        // of a Tag::Item that has not already received a TaskListMarker event (D-05, D-06).
                        let detected = if matches!(tag, Tag::Item)
                            && state.task_kind.is_empty()
                            && text_segments.is_empty()
                        {
                            extract_its_marker(&text)
                        } else {
                            None
                        };

                        if let Some((kind, remaining)) = detected {
                            state.task_kind.push(kind);
                            // Only add a text segment for the content after the marker (D-08)
                            if !remaining.is_empty() {
                                let mut seg = TextSegment::plain(remaining);
                                inline_styles.iter().for_each(|s| seg.add_style(s));
                                text_segments.push(InlineNode::Text(seg));
                            }
                        } else {
                            let mut text_segment = TextSegment::plain(&text);
                            inline_styles.iter().for_each(|style| {
                                text_segment.add_style(style);
                            });
                            text_segments.push(InlineNode::Text(text_segment));
                        }
                    }
                }

                Event::SoftBreak => {
                    // If inside a link accumulator, collect softbreak as space for the link
                    if let Some((_, ref mut text_pieces)) = link_accumulator {
                        text_pieces.push(" ".to_string());
                    } else {
                        let text_segment = TextSegment::empty_line();
                        text_segments.push(InlineNode::Text(text_segment));
                    }
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
                            let mut resolved_kind = kind.map(ast::BlockQuoteKind::from);
                            let mut title: Option<String> = None;

                            // ITS Theme detection: only when pulldown-cmark did not recognize the
                            // type (kind == None). Standard types (Note/Tip/etc.) are already
                            // consumed by pulldown-cmark and their [!TYPE] line is removed.
                            //
                            // When `> [!aside]\n> body text` is parsed, pulldown-cmark merges
                            // both lines into a single tight Paragraph with a SoftBreak event
                            // (which becomes '\n' in the RichText). We detect the [!type] pattern
                            // only on the first line, and if body content follows after '\n',
                            // we re-insert it as a new Paragraph node in the nodes list.
                            if resolved_kind.is_none() {
                                if let Some(first_text) =
                                    nodes.first().and_then(extract_paragraph_text)
                                {
                                    // Only examine the first line (before any SoftBreak newline).
                                    let (first_line, remainder) =
                                        match first_text.split_once('\n') {
                                            Some((first, rest)) => (first.trim(), rest.trim()),
                                            None => (first_text.trim(), ""),
                                        };
                                    if let Some(rest) = first_line.strip_prefix("[!") {
                                        if let Some(bracket_end) = rest.find(']') {
                                            let type_str = &rest[..bracket_end];
                                            let after_bracket =
                                                rest[bracket_end + 1..].trim().to_string();
                                            if let Some(detected_kind) = its_theme_kind(type_str) {
                                                resolved_kind = Some(detected_kind);
                                                title = if after_bracket.is_empty() {
                                                    None
                                                } else {
                                                    Some(after_bracket)
                                                };
                                                let first_range =
                                                    nodes[0].source_range().clone();
                                                nodes.remove(0);
                                                // Re-insert body content after the [!type] line
                                                // (merged by SoftBreak) as a new Paragraph node.
                                                if !remainder.is_empty() {
                                                    nodes.insert(
                                                        0,
                                                        Node::Paragraph {
                                                            text: RichText::from(
                                                                [TextSegment::plain(remainder)],
                                                            ),
                                                            source_range: first_range,
                                                        },
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            Some(Node::BlockQuote {
                                kind: resolved_kind,
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

    fn is_container_tag(tag: &Tag) -> bool {
        matches!(
            tag,
            Tag::Paragraph
                | Tag::Item
                | Tag::List(..)
                | Tag::BlockQuote(..)
                | Tag::CodeBlock(..)
                | Tag::Heading { .. }
                | Tag::Table(..)
                | Tag::FootnoteDefinition(..)
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
                Tag::Table(..) => Some(TagEnd::Table),
                Tag::FootnoteDefinition(..) => Some(TagEnd::FootnoteDefinition),
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

/// Maps an ITS Theme callout type string (case-insensitive) to a [`ast::BlockQuoteKind`] variant.
/// Returns [`None`] for unrecognized type strings (treated as plain blockquotes).
/// Handles aliases: caption/captions, column/columns, quote/quotes.
fn its_theme_kind(type_str: &str) -> Option<ast::BlockQuoteKind> {
    match type_str.to_ascii_lowercase().as_str() {
        // Standard GitHub Alert types (for case-insensitive support)
        "note" => Some(ast::BlockQuoteKind::Note),
        "tip" => Some(ast::BlockQuoteKind::Tip),
        "important" => Some(ast::BlockQuoteKind::Important),
        "warning" => Some(ast::BlockQuoteKind::Warning),
        "caution" => Some(ast::BlockQuoteKind::Caution),
        // ITS Theme Extended (post-processing detection)
        "aside" => Some(ast::BlockQuoteKind::Aside),
        "blank" => Some(ast::BlockQuoteKind::Blank),
        "caption" | "captions" => Some(ast::BlockQuoteKind::Caption),
        "cards" => Some(ast::BlockQuoteKind::Cards),
        "checks" => Some(ast::BlockQuoteKind::Checks),
        "column" | "columns" => Some(ast::BlockQuoteKind::Column),
        "grid" => Some(ast::BlockQuoteKind::Grid),
        "infobox" => Some(ast::BlockQuoteKind::Infobox),
        "kanban" => Some(ast::BlockQuoteKind::Kanban),
        "kith" => Some(ast::BlockQuoteKind::Kith),
        "metadata" => Some(ast::BlockQuoteKind::Metadata),
        "quote" | "quotes" => Some(ast::BlockQuoteKind::Quote),
        "recite" => Some(ast::BlockQuoteKind::Recite),
        "statblocks" => Some(ast::BlockQuoteKind::Statblocks),
        "timeline" => Some(ast::BlockQuoteKind::Timeline),
        _ => None,
    }
}

/// Extracts the text content of a [`Node::Paragraph`] as a [`String`].
/// Returns [`None`] if the node is not a paragraph.
fn extract_paragraph_text(node: &ast::Node) -> Option<String> {
    if let ast::Node::Paragraph { text, .. } = node {
        Some(text.to_string())
    } else {
        None
    }
}

/// Attempts to extract an ITS Theme task marker from the start of a text segment.
///
/// Matches the pattern `[char]` at the beginning of `text` (after optional leading whitespace),
/// where `char` is exactly one character from the 35 ITS Theme marker set.
///
/// Returns `Some((TaskKind, remaining))` where `remaining` is the text after the `[char]` marker
/// and any immediately following whitespace. Returns `None` if the text does not start with a
/// recognized `[char]` pattern.
fn extract_its_marker(text: &str) -> Option<(ast::TaskKind, &str)> {
    let trimmed = text.trim_start();

    // Must start with '['
    let rest = trimmed.strip_prefix('[')?;

    // Isolate the single char between '[' and ']'
    let mut chars = rest.chars();
    let marker_char = chars.next()?;
    let after_char = chars.as_str();

    // Next char must be ']'
    let after_bracket = after_char.strip_prefix(']')?;

    // Map the marker char to a TaskKind variant (case-sensitive — [C] ≠ [c])
    let kind = match marker_char {
        '-' => ast::TaskKind::Dropped,
        '>' => ast::TaskKind::Forward,
        '<' => ast::TaskKind::Migrated,
        'D' => ast::TaskKind::Date,
        '?' => ast::TaskKind::Question,
        '/' => ast::TaskKind::HalfDone,
        '+' => ast::TaskKind::Add,
        'R' => ast::TaskKind::Research,
        '!' => ast::TaskKind::Important,
        'i' => ast::TaskKind::Idea,
        'B' => ast::TaskKind::Brainstorm,
        'P' => ast::TaskKind::Pro,
        'C' => ast::TaskKind::Con,
        'Q' => ast::TaskKind::Quote,
        'N' => ast::TaskKind::Note,
        'b' => ast::TaskKind::Bookmark,
        'I' => ast::TaskKind::Information,
        'p' => ast::TaskKind::Paraphrase,
        'L' => ast::TaskKind::Location,
        'E' => ast::TaskKind::Example,
        'A' => ast::TaskKind::Answer,
        'r' => ast::TaskKind::Reward,
        'c' => ast::TaskKind::Choice,
        'd' => ast::TaskKind::Doing,
        'T' => ast::TaskKind::Time,
        '@' => ast::TaskKind::Character,
        't' => ast::TaskKind::Talk,
        'O' => ast::TaskKind::Outline,
        '~' => ast::TaskKind::Conflict,
        'W' => ast::TaskKind::World,
        'f' => ast::TaskKind::Clue,
        'F' => ast::TaskKind::Foreshadow,
        'H' => ast::TaskKind::Favorite,
        '&' => ast::TaskKind::Symbolism,
        's' => ast::TaskKind::Secret,
        // Unknown char: LooselyChecked fallback stores the original char (D-04)
        c => ast::TaskKind::LooselyChecked(c),
    };

    // Strip one leading space after `]` if present (e.g., "[>] text" → "text")
    let remaining = after_bracket.strip_prefix(' ').unwrap_or(after_bracket);
    Some((kind, remaining))
}

pub fn from_str(text: &str) -> Vec<Node> {
    Parser::new(text).parse()
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
            // TODO: Implement horizontal rule
            // (
            //     "horizontal_rule",
            //     indoc! { r#"## Horizontal rule
            //     You can use three or more stars `***`, hyphens `---`, or underscore `___` on its own line to add a horizontal bar. You can also separate symbols using spaces.
            //
            //     ***
            //     ****
            //     * * *
            //     ---
            //     ----
            //     - - -
            //     ___
            //     ____
            //     _ _ _
            //     "#},
            // ),
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

    /// Parse a blockquote input and return (kind, title, body_texts).
    fn parse_bq(input: &str) -> (Option<ast::BlockQuoteKind>, Option<String>, Vec<String>) {
        let nodes = from_str(input);
        assert_eq!(nodes.len(), 1, "expected one top-level node");
        match nodes.into_iter().next().unwrap() {
            Node::BlockQuote { kind, title, nodes, .. } => {
                let texts = nodes
                    .into_iter()
                    .filter_map(|n| match n {
                        Node::Paragraph { text, .. } => Some(text.to_string()),
                        _ => None,
                    })
                    .collect();
                (kind, title, texts)
            }
            _ => panic!("expected BlockQuote"),
        }
    }

    #[test]
    fn its_theme_aside_callout() {
        let (kind, title, body) = parse_bq("> [!aside]\n> body text");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Aside));
        assert_eq!(title, None);
        assert_eq!(body, vec!["body text"]);
    }

    #[test]
    fn its_theme_kanban_with_title() {
        let (kind, title, body) = parse_bq("> [!kanban] My Board\n> body text");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Kanban));
        assert_eq!(title, Some("My Board".to_string()));
        assert_eq!(body, vec!["body text"]);
    }

    #[test]
    fn its_theme_case_insensitive() {
        let (kind, title, body) = parse_bq("> [!ASIDE]\n> body");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Aside));
        assert_eq!(title, None);
        assert_eq!(body, vec!["body"]);
    }

    #[test]
    fn its_theme_aliases() {
        let (kind, _, _) = parse_bq("> [!captions]\n> text");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Caption));

        let (kind2, _, _) = parse_bq("> [!columns]\n> text");
        assert_eq!(kind2, Some(ast::BlockQuoteKind::Column));

        let (kind3, _, _) = parse_bq("> [!quotes]\n> text");
        assert_eq!(kind3, Some(ast::BlockQuoteKind::Quote));
    }

    #[test]
    fn plain_blockquote_unchanged() {
        let (kind, title, _) = parse_bq("> plain text");
        assert_eq!(kind, None);
        assert_eq!(title, None);
    }

    #[test]
    fn unknown_type_stays_plain() {
        let (kind, title, _) = parse_bq("> [!unknowntype]\n> body");
        assert_eq!(kind, None);
        assert_eq!(title, None);
    }

    #[test]
    fn standard_callout_kind_unchanged() {
        // pulldown-cmark natively recognizes [!tip], kind is Some(Tip)
        let (kind, title, body) = parse_bq("> [!tip]\n> body text");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Tip));
        assert_eq!(title, None);
        assert_eq!(body, vec!["body text"]);
    }

    #[test]
    fn standard_callout_case_insensitive() {
        // Verify ITS Theme detection handles lowercase standard types.
        let (kind, title, body) = parse_bq("> [!note]\n> body text");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Note));
        assert_eq!(title, None);
        assert_eq!(body, vec!["body text"]);
    }

    #[test]
    fn standard_callout_all_types_support() {
        // Test all 5 standard types with lowercase.
        let (kind, _, _) = parse_bq("> [!note]\n> body");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Note));

        let (kind, _, _) = parse_bq("> [!tip]\n> body");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Tip));

        let (kind, _, _) = parse_bq("> [!important]\n> body");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Important));

        let (kind, _, _) = parse_bq("> [!warning]\n> body");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Warning));

        let (kind, _, _) = parse_bq("> [!caution]\n> body");
        assert_eq!(kind, Some(ast::BlockQuoteKind::Caution));
    }

    #[test]
    fn test_extract_its_marker_known() {
        assert_eq!(
            extract_its_marker("[>] Schedule this"),
            Some((ast::TaskKind::Forward, "Schedule this"))
        );
        assert_eq!(
            extract_its_marker("[!] Critical"),
            Some((ast::TaskKind::Important, "Critical"))
        );
        assert_eq!(
            extract_its_marker("[C] Against"),
            Some((ast::TaskKind::Con, "Against"))
        );
        assert_eq!(
            extract_its_marker("[c] Pick one"),
            Some((ast::TaskKind::Choice, "Pick one"))
        );
        assert_eq!(
            extract_its_marker("[-] Dropped task"),
            Some((ast::TaskKind::Dropped, "Dropped task"))
        );
        assert_eq!(
            extract_its_marker("[?] A question"),
            Some((ast::TaskKind::Question, "A question"))
        );
    }

    #[test]
    fn test_extract_its_marker_loosely_checked() {
        // Unknown char → LooselyChecked(char)
        assert_eq!(
            extract_its_marker("[z] Unknown"),
            Some((ast::TaskKind::LooselyChecked('z'), "Unknown"))
        );
        assert_eq!(
            extract_its_marker("[y]"),
            Some((ast::TaskKind::LooselyChecked('y'), ""))
        );
    }

    #[test]
    fn test_extract_its_marker_no_match() {
        // Not a [char] pattern
        assert_eq!(extract_its_marker("plain text"), None);
        assert_eq!(extract_its_marker("[no closing bracket"), None);
        // Multi-char inside brackets → None (not a single char pattern)
        assert_eq!(extract_its_marker("[ab] text"), None);
    }

    #[test]
    fn test_parse_its_theme_task_items() {
        let md = indoc! {"
            - [>] Forwarded item
            - [!] Important item
            - [c] Choice item
        "};
        let nodes = from_str(md);
        // Should produce 1 List with 3 Task nodes
        assert_eq!(nodes.len(), 1, "expected one top-level List node");
        if let Node::List { nodes: items, .. } = &nodes[0] {
            assert_eq!(items.len(), 3);
            if let Node::Task { kind, .. } = &items[0] {
                assert_eq!(*kind, ast::TaskKind::Forward, "first item should be Forward");
            } else {
                panic!("expected Task node for [>] item");
            }
            if let Node::Task { kind, .. } = &items[1] {
                assert_eq!(*kind, ast::TaskKind::Important, "second item should be Important");
            } else {
                panic!("expected Task node for [!] item");
            }
            if let Node::Task { kind, .. } = &items[2] {
                assert_eq!(*kind, ast::TaskKind::Choice, "third item should be Choice");
            } else {
                panic!("expected Task node for [c] item");
            }
        } else {
            panic!("expected List node");
        }
    }

    #[test]
    fn test_parse_table() {
        let md = indoc! {"
            | Name  | Age | City   |
            | :---- | --: | :----: |
            | Alice |  28 | London |
            | Bob   |  34 | Paris  |
        "};
        let nodes = from_str(md);
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            Node::Table { alignments, header, rows, .. } => {
                assert_eq!(alignments.len(), 3);
                assert_eq!(alignments[0], ast::Alignment::Left);
                assert_eq!(alignments[1], ast::Alignment::Right);
                assert_eq!(alignments[2], ast::Alignment::Center);
                assert_eq!(header.len(), 3);
                assert_eq!(rows.len(), 2);
            }
            other => panic!("expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_standard_markers_unaffected() {
        // Standard markers ([x], [ ]) should still use TaskListMarker path
        let md = "- [x] Done\n- [ ] Todo\n";
        let nodes = from_str(md);
        assert_eq!(nodes.len(), 1);
        if let Node::List { nodes: items, .. } = &nodes[0] {
            assert_eq!(items.len(), 2);
            if let Node::Task { kind, .. } = &items[0] {
                assert_eq!(*kind, ast::TaskKind::Checked);
            }
            if let Node::Task { kind, .. } = &items[1] {
                assert_eq!(*kind, ast::TaskKind::Unchecked);
            }
        }
    }

    // === Phase 7 Footnote tests (Wave 2 — 07-02 implementation) ===

    #[test]
    fn test_parse_footnote_reference() {
        let nodes = Parser::new("Hello [^1] world").parse();
        let sexp = ast::nodes_to_sexp(&nodes, 0);
        assert_snapshot!(sexp);
    }

    #[test]
    fn test_parse_footnote_section() {
        let input = indoc! { r#"
        Text with a reference [^1] here.

        [^1]: This is the footnote definition.
        "#};
        let nodes = Parser::new(input).parse();
        let sexp = ast::nodes_to_sexp(&nodes, 0);
        assert_snapshot!(sexp);
    }

    #[test]
    fn test_parse_multiple_footnotes() {
        let input = indoc! { r#"
        First [^1] and second [^2].

        [^1]: First definition.
        [^2]: Second definition.
        "#};
        let nodes = Parser::new(input).parse();
        let sexp = ast::nodes_to_sexp(&nodes, 0);
        assert_snapshot!(sexp);
    }

    // === Phase 7 External link tests (Wave 3 — 07-03 implementation) ===

    #[test]
    fn test_parse_link() {
        use crate::note_editor::rich_text::LinkTarget;

        let nodes = Parser::new("Click [here](https://example.com) now").parse();
        let sexp = ast::nodes_to_sexp(&nodes, 0);
        assert_snapshot!(sexp);
        // Verify the link node structure directly
        assert_eq!(nodes.len(), 1);
        if let Node::Paragraph { text, .. } = &nodes[0] {
            let nodes = text.nodes();
            assert_eq!(nodes.len(), 3);
            // Check first and third are regular text
            assert!(matches!(nodes[0], InlineNode::Text(..)));
            assert!(matches!(nodes[2], InlineNode::Text(..)));
            // Check middle is a link with correct text and target
            if let InlineNode::Link { text, target } = &nodes[1] {
                assert_eq!(text, "here");
                assert_eq!(target, &LinkTarget::External("https://example.com".to_string()));
            } else {
                panic!("Expected InlineNode::Link");
            }
        } else {
            panic!("Expected Paragraph node");
        }
    }

    #[test]
    fn test_parse_non_http_link() {
        let nodes = Parser::new("Mail [me](mailto:a@b.com)").parse();
        let sexp = ast::nodes_to_sexp(&nodes, 0);
        assert_snapshot!(sexp);
        // Verify mailto links render as plain text, not InlineNode::Link
        if let Node::Paragraph { text, .. } = &nodes[0] {
            assert!(text.nodes().iter().all(|n| !matches!(n, InlineNode::Link { .. })));
        } else {
            panic!("Expected Paragraph node");
        }
    }
}
