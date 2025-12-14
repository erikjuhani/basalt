use std::ops::{Deref, DerefMut};

use pulldown_cmark::{CodeBlockKind, Event, Options, Tag, TagEnd};

use crate::note_editor::{
    ast::{self, Node, SourceRange, TaskKind},
    rich_text::{RichText, Style, TextSegment},
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
    task_kind: Option<ast::TaskKind>,
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
        let parser = pulldown_cmark::TextMergeWithOffset::new(
            pulldown_cmark::Parser::new_ext(text, Options::all()).into_offset_iter(),
        );

        Self(parser)
    }

    pub fn parse(mut self) -> Vec<Node> {
        let mut result = Vec::new();
        let mut state = ParserState::default();

        while let Some((event, _)) = self.next() {
            match event {
                Event::Start(tag) if Self::is_container_tag(&tag) => {
                    if let Some(node) = self.parse_container(tag, &mut state) {
                        result.push(node);
                    }
                }
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

                Event::TaskListMarker(checked) => {
                    state.task_kind = Some(if checked {
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

                            let item = if let Some(kind) = state.task_kind.take() {
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
}
