use std::{borrow::Cow, cell::RefCell, ops::Range, sync::LazyLock};

use ratatui::style::Color;
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

use crate::config::theme::Syntax;

/// Semantic class of a code token. The theme's `[syntax]` roles give each
/// kind its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    String,
    Comment,
    Function,
    Type,
    Constant,
}

impl TokenKind {
    pub fn color(self, syntax: &Syntax) -> Color {
        match self {
            TokenKind::Keyword => syntax.keyword,
            TokenKind::String => syntax.string,
            TokenKind::Comment => syntax.comment,
            TokenKind::Function => syntax.function,
            TokenKind::Type => syntax.type_name,
            TokenKind::Constant => syntax.constant,
        }
    }
}

/// A range of one line that shares a token kind, `None` for plain code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub range: Range<usize>,
    pub kind: Option<TokenKind>,
}

/// Tokens of one line, in source order.
pub type LineTokens = Vec<Token>;

/// Grammar capture names recognised by every language below, and the token
/// class each one renders as. A grammar's own capture names, such as
/// `constant.builtin` or `function.macro`, resolve to their nearest prefix in
/// this list, so only the roots need listing here.
const CAPTURE_TOKENS: &[(&str, TokenKind)] = &[
    ("comment", TokenKind::Comment),
    ("string", TokenKind::String),
    ("constant", TokenKind::Constant),
    ("number", TokenKind::Constant),
    ("boolean", TokenKind::Constant),
    ("escape", TokenKind::Constant),
    ("keyword", TokenKind::Keyword),
    ("function", TokenKind::Function),
    ("type", TokenKind::Type),
];

fn kind_for(Highlight(index): Highlight) -> Option<TokenKind> {
    CAPTURE_TOKENS.get(index).map(|(_, token)| *token)
}

/// The curated set of languages basalt highlights fenced code blocks in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Language {
    Rust,
    Toml,
    Json,
    Bash,
    JavaScript,
    TypeScript,
    Python,
    Go,
    Yaml,
    C,
}

struct Grammar {
    language: tree_sitter::Language,
    name: &'static str,
    highlights_query: Cow<'static, str>,
}

impl Grammar {
    fn new(
        language: impl Into<tree_sitter::Language>,
        name: &'static str,
        highlights_query: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            language: language.into(),
            name,
            highlights_query: highlights_query.into(),
        }
    }

    fn compile(self) -> HighlightConfiguration {
        let mut configuration =
            HighlightConfiguration::new(self.language, self.name, &self.highlights_query, "", "")
                .unwrap_or_else(|error| panic!("invalid {} highlight query: {error}", self.name));
        let names: Vec<&str> = CAPTURE_TOKENS.iter().map(|(capture, _)| *capture).collect();
        configuration.configure(&names);
        configuration
    }
}

impl Language {
    const ALL: [Language; 10] = [
        Language::Rust,
        Language::Toml,
        Language::Json,
        Language::Bash,
        Language::JavaScript,
        Language::TypeScript,
        Language::Python,
        Language::Go,
        Language::Yaml,
        Language::C,
    ];

    fn from_token(word: &str) -> Option<Self> {
        match word {
            "rust" | "rs" => Some(Self::Rust),
            "toml" => Some(Self::Toml),
            "json" => Some(Self::Json),
            "bash" | "sh" | "shell" | "zsh" => Some(Self::Bash),
            "javascript" | "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "typescript" | "ts" => Some(Self::TypeScript),
            "python" | "py" => Some(Self::Python),
            "go" | "golang" => Some(Self::Go),
            "yaml" | "yml" => Some(Self::Yaml),
            "c" => Some(Self::C),
            _ => None,
        }
    }

    fn grammar(self) -> Grammar {
        match self {
            Language::Rust => Grammar::new(
                tree_sitter_rust::LANGUAGE,
                "rust",
                tree_sitter_rust::HIGHLIGHTS_QUERY,
            ),
            Language::Toml => Grammar::new(
                tree_sitter_toml_ng::LANGUAGE,
                "toml",
                tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
            ),
            Language::Json => Grammar::new(
                tree_sitter_json::LANGUAGE,
                "json",
                tree_sitter_json::HIGHLIGHTS_QUERY,
            ),
            Language::Bash => Grammar::new(
                tree_sitter_bash::LANGUAGE,
                "bash",
                tree_sitter_bash::HIGHLIGHT_QUERY,
            ),
            Language::JavaScript => Grammar::new(
                tree_sitter_javascript::LANGUAGE,
                "javascript",
                tree_sitter_javascript::HIGHLIGHT_QUERY,
            ),
            Language::TypeScript => Grammar::new(
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
                "typescript",
                format!(
                    "{}{}",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_typescript::HIGHLIGHTS_QUERY
                ),
            ),
            Language::Python => Grammar::new(
                tree_sitter_python::LANGUAGE,
                "python",
                tree_sitter_python::HIGHLIGHTS_QUERY,
            ),
            Language::Go => Grammar::new(
                tree_sitter_go::LANGUAGE,
                "go",
                tree_sitter_go::HIGHLIGHTS_QUERY,
            ),
            Language::Yaml => Grammar::new(
                tree_sitter_yaml::LANGUAGE,
                "yaml",
                tree_sitter_yaml::HIGHLIGHTS_QUERY,
            ),
            Language::C => {
                Grammar::new(tree_sitter_c::LANGUAGE, "c", tree_sitter_c::HIGHLIGHT_QUERY)
            }
        }
    }

    fn configuration(self) -> Option<&'static HighlightConfiguration> {
        LANGUAGES
            .iter()
            .find(|(language, _)| *language == self)
            .map(|(_, configuration)| configuration)
    }
}

static LANGUAGES: LazyLock<[(Language, HighlightConfiguration); 10]> =
    LazyLock::new(|| Language::ALL.map(|language| (language, language.grammar().compile())));

thread_local! {
    /// One highlighter per thread, reused across blocks so its parser and
    /// query cursors stay allocated, as the crate recommends.
    static HIGHLIGHTER: RefCell<Highlighter> = RefCell::new(Highlighter::new());
}

/// Byte ranges of `text`'s lines, terminator excluded. A text ending in `\n`
/// has a trailing empty range, unlike `str::lines()`, so a caller that joined
/// lines with `\n` gets one range back per line.
fn line_ranges(text: &str) -> Vec<Range<usize>> {
    let mut start = 0;
    text.split('\n')
        .map(|line| {
            let end = start + line.strip_suffix('\r').unwrap_or(line).len();
            let range = start..end;
            start += line.len() + 1;
            range
        })
        .collect()
}

/// TokenKind ranges under construction, one entry per line of the block.
/// Highlight events arrive in source order, so `current` only moves forward.
struct BlockLines {
    ranges: Vec<Range<usize>>,
    tokens: Vec<LineTokens>,
    current: usize,
}

impl BlockLines {
    fn new(text: &str) -> Self {
        let ranges = line_ranges(text);
        let tokens = vec![LineTokens::new(); ranges.len()];
        Self {
            ranges,
            tokens,
            current: 0,
        }
    }

    /// Splits one source range across the lines it spans.
    fn push(&mut self, source: Range<usize>, kind: Option<TokenKind>) {
        while let Some(range) = self.ranges.get(self.current) {
            if source.start >= range.end {
                self.current += 1;
                continue;
            }
            let clipped = source.start.max(range.start)..source.end.min(range.end);
            if !clipped.is_empty() {
                let line_local = (clipped.start - range.start)..(clipped.end - range.start);
                push_token(&mut self.tokens[self.current], line_local, kind);
            }
            if source.end <= range.end {
                break;
            }
            self.current += 1;
        }
    }

    /// One entry per line. A line without tokens gets a single empty range,
    /// so the renderer always has one span to hang the newline byte on.
    fn finish(self) -> Vec<LineTokens> {
        self.tokens
            .into_iter()
            .map(|line| match line.is_empty() {
                true => vec![Token {
                    range: 0..0,
                    kind: None,
                }],
                false => line,
            })
            .collect()
    }
}

/// TokenKind ranges for every line of a fenced code block, one language parse for
/// the whole block. `None` when no grammar matches `info`, or when the grammar
/// fails on `text`, so the caller renders the block as plain code.
pub fn block_tokens(info: &str, text: &str) -> Option<Vec<LineTokens>> {
    let language = info
        .split_whitespace()
        .next()
        .and_then(Language::from_token)?;
    let configuration = language.configuration()?;
    HIGHLIGHTER.with_borrow_mut(|highlighter| {
        let events = highlighter
            .highlight(configuration, text.as_bytes(), None, None, |_| None)
            .ok()?;

        let mut lines = BlockLines::new(text);
        let mut active: Vec<Highlight> = Vec::new();
        for event in events {
            match event.ok()? {
                HighlightEvent::HighlightStart(highlight) => active.push(highlight),
                HighlightEvent::HighlightEnd => {
                    active.pop();
                }
                HighlightEvent::Source { start, end } => {
                    lines.push(start..end, active.last().copied().and_then(kind_for));
                }
            }
        }
        Some(lines.finish())
    })
}

/// Appends a token, or extends the last one when it has the same kind.
fn push_token(tokens: &mut LineTokens, range: Range<usize>, kind: Option<TokenKind>) {
    match tokens.last_mut() {
        Some(last) if last.kind == kind => last.range.end = range.end,
        _ => tokens.push(Token { range, kind }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_lines_exclude_the_carriage_return() {
        assert_eq!(line_ranges("a\r\nb\n"), vec![0..1, 3..4, 5..5]);
    }

    fn tokens<'a>(line: &'a str, tokens: &[Token]) -> Vec<(&'a str, Option<TokenKind>)> {
        tokens
            .iter()
            .map(|token| (&line[token.range.clone()], token.kind))
            .collect()
    }

    fn plain_empty() -> Token {
        Token {
            range: 0..0,
            kind: None,
        }
    }

    #[test]
    fn classifies_javascript_tokens() {
        let line = "const answer = 42; // done";
        let lines = block_tokens("js", line).unwrap();
        assert_eq!(
            tokens(line, &lines[0]),
            vec![
                ("const", Some(TokenKind::Keyword)),
                (" answer = ", None),
                ("42", Some(TokenKind::Constant)),
                ("; ", None),
                ("// done", Some(TokenKind::Comment)),
            ]
        );
    }

    #[test]
    fn word_operators_are_keywords() {
        let line = "typeof x";
        let lines = block_tokens("js", line).unwrap();
        assert_eq!(
            tokens(line, &lines[0])[0],
            ("typeof", Some(TokenKind::Keyword))
        );
    }

    #[test]
    fn classifies_rust_function_and_string() {
        let line = r#"fn greet() { println!("hi"); }"#;
        let lines = block_tokens("rust", line).unwrap();
        let segments = tokens(line, &lines[0]);
        assert!(segments.contains(&("fn", Some(TokenKind::Keyword))));
        assert!(segments.contains(&("greet", Some(TokenKind::Function))));
        assert!(segments.contains(&("\"hi\"", Some(TokenKind::String))));
    }

    #[test]
    fn multi_line_comment_carries_across_lines() {
        let text = "/* open\nstill comment */ let x = 1;";
        let lines = block_tokens("rust", text).unwrap();
        assert_eq!(
            tokens("still comment */ let x = 1;", &lines[1])[0],
            ("still comment */", Some(TokenKind::Comment))
        );
    }

    #[test]
    fn empty_line_yields_one_empty_range() {
        let lines = block_tokens("rust", "").unwrap();
        assert_eq!(lines, vec![vec![plain_empty()]]);
    }

    #[test]
    fn one_entry_per_line_including_a_trailing_blank_one() {
        let source_lines = ["const a = 1;", ""];
        let lines = block_tokens("js", &source_lines.join("\n")).unwrap();
        assert_eq!(lines.len(), source_lines.len());
        assert_eq!(lines[1], vec![plain_empty()]);
    }

    #[test]
    fn info_string_takes_the_first_word() {
        assert!(block_tokens("py title=example.py", "x = 1").is_some());
        assert!(block_tokens("", "x").is_none());
        assert!(block_tokens("no-such-language", "x").is_none());
    }

    #[test]
    fn block_tokens_parses_every_line_once() {
        let ranges = block_tokens("js", "const a = 1;\nconst b = 2;").unwrap();
        assert_eq!(ranges.len(), 2);
        assert_eq!(
            tokens("const a = 1;", &ranges[0])[0],
            ("const", Some(TokenKind::Keyword))
        );
    }

    #[test]
    fn block_tokens_is_none_for_unknown_language() {
        assert!(block_tokens("no-such-language", "a").is_none());
    }
}
