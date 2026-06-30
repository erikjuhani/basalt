use std::ops::{Deref, DerefMut};

use pulldown_cmark::{CodeBlockKind, Event, Options, Tag, TagEnd};

use crate::note_editor::{
    ast::{self, Node, SourceRange, TaskKind},
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
                        Tag::BlockQuote(kind) => Some(Node::BlockQuote {
                            kind: kind.map(|kind| kind.into()),
                            nodes,
                            source_range,
                        }),
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
}
