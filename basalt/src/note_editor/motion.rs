use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Blank,
    Word,
    Punct,
}

fn class(c: char, big: bool) -> Class {
    if c.is_whitespace() {
        Class::Blank
    } else if !(big || c.is_alphanumeric() || c == '_') {
        Class::Punct
    } else {
        Class::Word
    }
}

fn chars_from(content: &str, offset: usize) -> impl Iterator<Item = (usize, char)> + '_ {
    content[offset..]
        .char_indices()
        .map(move |(i, c)| (offset + i, c))
}

fn char_len_at(content: &str, offset: usize) -> usize {
    content[offset..].chars().next().map_or(0, char::len_utf8)
}

fn prev_char_offset(content: &str, offset: usize) -> usize {
    content[..offset]
        .char_indices()
        .next_back()
        .map_or(offset, |(i, _)| i)
}

pub fn line_start(content: &str, offset: usize) -> usize {
    content[..offset].rfind('\n').map_or(0, |i| i + 1)
}

pub fn line_end_exclusive(content: &str, offset: usize) -> usize {
    content[offset..]
        .find('\n')
        .map_or(content.len(), |i| offset + i)
}

pub fn line_end(content: &str, offset: usize) -> usize {
    let start = line_start(content, offset);
    let end = line_end_exclusive(content, offset);
    content[start..end]
        .char_indices()
        .next_back()
        .map_or(start, |(i, _)| start + i)
}

pub fn first_nonblank(content: &str, offset: usize) -> usize {
    let start = line_start(content, offset);
    let end = line_end_exclusive(content, offset);
    content[start..end]
        .char_indices()
        .find(|&(_, c)| !c.is_whitespace())
        .map_or(start, |(i, _)| start + i)
}

pub fn word_forward(content: &str, offset: usize, big: bool) -> usize {
    let mut chars = chars_from(content, offset).peekable();
    let Some(&(_, first)) = chars.peek() else {
        return offset;
    };

    let start = class(first, big);
    if start != Class::Blank {
        while let Some(&(_, c)) = chars.peek() {
            if class(c, big) == start && c != '\n' {
                chars.next();
            } else {
                break;
            }
        }
    }

    // An empty line counts as a word.
    while let Some(&(_, c)) = chars.peek() {
        if c == '\n' {
            chars.next();
            if let Some(&(pos, '\n')) = chars.peek() {
                return pos;
            }
        } else if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    chars.peek().map_or(content.len(), |&(pos, _)| pos)
}

pub fn word_backward(content: &str, offset: usize, big: bool) -> usize {
    let mut chars = content[..offset].char_indices().rev().peekable();

    while let Some(&(_, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    let Some(&(mut start, c)) = chars.peek() else {
        return 0;
    };
    let target = class(c, big);
    while let Some(&(i, c)) = chars.peek() {
        if class(c, big) == target && c != '\n' {
            start = i;
            chars.next();
        } else {
            break;
        }
    }
    start
}

pub fn word_end(content: &str, offset: usize, big: bool) -> usize {
    let mut chars = chars_from(content, offset).skip(1).peekable();

    while let Some(&(_, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    let Some(&(mut end, c)) = chars.peek() else {
        return prev_char_offset(content, content.len()).max(offset);
    };
    let target = class(c, big);
    while let Some(&(i, c)) = chars.peek() {
        if class(c, big) == target && c != '\n' {
            end = i;
            chars.next();
        } else {
            break;
        }
    }
    end
}

pub fn find_char(
    content: &str,
    offset: usize,
    target: char,
    forward: bool,
    till: bool,
) -> Option<usize> {
    if forward {
        let start = offset + char_len_at(content, offset);
        let end = line_end_exclusive(content, offset);
        let found = content
            .get(start..end)?
            .char_indices()
            .find(|&(_, c)| c == target)
            .map(|(i, _)| start + i)?;
        Some(if till {
            prev_char_offset(content, found)
        } else {
            found
        })
    } else {
        let start = line_start(content, offset);
        let found = content
            .get(start..offset)?
            .char_indices()
            .rev()
            .find(|&(_, c)| c == target)
            .map(|(i, _)| start + i)?;
        Some(if till {
            found + char_len_at(content, found)
        } else {
            found
        })
    }
}

fn is_empty_line(content: &str, line_start: usize) -> bool {
    line_start >= content.len() || content[line_start..].starts_with('\n')
}

fn next_line_start(content: &str, offset: usize) -> usize {
    content[offset..]
        .find('\n')
        .map_or(content.len(), |i| offset + i + 1)
}

fn prev_line_start(content: &str, line_start: usize) -> usize {
    if line_start == 0 {
        return 0;
    }
    content[..line_start - 1].rfind('\n').map_or(0, |i| i + 1)
}

pub fn paragraph_forward(content: &str, offset: usize) -> usize {
    let mut line = next_line_start(content, offset);
    while line < content.len() {
        if is_empty_line(content, line) {
            return line;
        }
        line = next_line_start(content, line);
    }
    content.len()
}

pub fn paragraph_backward(content: &str, offset: usize) -> usize {
    let mut line = line_start(content, offset);
    while line > 0 {
        line = prev_line_start(content, line);
        if is_empty_line(content, line) {
            return line;
        }
    }
    0
}

const PAIRS: [(char, char); 3] = [('(', ')'), ('[', ']'), ('{', '}')];

pub fn matching_pair(content: &str, offset: usize) -> Option<usize> {
    let end = line_end_exclusive(content, offset);
    let (bracket_offset, bracket) = content
        .get(offset..end)?
        .char_indices()
        .map(|(i, c)| (offset + i, c))
        .find(|&(_, c)| "()[]{}".contains(c))?;

    let (open, close, forward) = PAIRS.iter().find_map(|&(open, close)| {
        if bracket == open {
            Some((open, close, true))
        } else if bracket == close {
            Some((open, close, false))
        } else {
            None
        }
    })?;

    let mut depth = 0i32;
    if forward {
        for (i, c) in chars_from(content, bracket_offset) {
            depth += (c == open) as i32 - (c == close) as i32;
            if depth == 0 {
                return Some(i);
            }
        }
    } else {
        for (i, c) in content[..bracket_offset + 1].char_indices().rev() {
            depth += (c == close) as i32 - (c == open) as i32;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn char_at(content: &str, offset: usize) -> Option<char> {
    content[offset..].chars().next()
}

pub fn text_object(
    content: &str,
    offset: usize,
    object: char,
    around: bool,
) -> Option<Range<usize>> {
    match object {
        'w' => Some(word_object(content, offset, false, around)),
        'W' => Some(word_object(content, offset, true, around)),
        '"' | '\'' | '`' => quote_object(content, offset, object, around),
        '(' | ')' | 'b' => pair_object(content, offset, '(', ')', around),
        '[' | ']' => pair_object(content, offset, '[', ']', around),
        '{' | '}' | 'B' => pair_object(content, offset, '{', '}', around),
        '<' | '>' => pair_object(content, offset, '<', '>', around),
        _ => None,
    }
}

fn word_object(content: &str, offset: usize, big: bool, around: bool) -> Range<usize> {
    let Some(cursor) = char_at(content, offset) else {
        return offset..offset;
    };
    let target = class(cursor, big);

    let mut start = offset;
    loop {
        let prev = prev_char_offset(content, start);
        match char_at(content, prev) {
            Some(c) if prev != start && c != '\n' && class(c, big) == target => start = prev,
            _ => break,
        }
    }

    let mut end = offset;
    while let Some(c) = char_at(content, end) {
        if c == '\n' || class(c, big) != target {
            break;
        }
        end += c.len_utf8();
    }

    if !around || target == Class::Blank {
        return start..end;
    }

    let mut around_end = end;
    while let Some(c) = char_at(content, around_end).filter(|&c| c == ' ' || c == '\t') {
        around_end += c.len_utf8();
    }
    if around_end > end {
        return start..around_end;
    }

    let mut around_start = start;
    loop {
        let prev = prev_char_offset(content, around_start);
        match char_at(content, prev) {
            Some(c) if prev != around_start && (c == ' ' || c == '\t') => around_start = prev,
            _ => break,
        }
    }
    around_start..end
}

fn quote_object(content: &str, offset: usize, quote: char, around: bool) -> Option<Range<usize>> {
    let start = line_start(content, offset);
    let end = line_end_exclusive(content, offset);
    let quotes: Vec<usize> = content[start..end]
        .char_indices()
        .filter(|&(_, c)| c == quote)
        .map(|(i, _)| start + i)
        .collect();

    let (open, close) = quotes
        .chunks_exact(2)
        .map(|pair| (pair[0], pair[1]))
        .find(|&(_, close)| offset <= close)?;

    if around {
        Some(open..close + quote.len_utf8())
    } else {
        Some(open + quote.len_utf8()..close)
    }
}

fn pair_object(
    content: &str,
    offset: usize,
    open: char,
    close: char,
    around: bool,
) -> Option<Range<usize>> {
    let open_pos = enclosing_open(content, offset, open, close)?;
    let close_pos = matching_close(content, open_pos, open, close)?;
    if around {
        Some(open_pos..close_pos + close.len_utf8())
    } else {
        Some(open_pos + open.len_utf8()..close_pos)
    }
}

fn enclosing_open(content: &str, offset: usize, open: char, close: char) -> Option<usize> {
    let end = offset + char_len_at(content, offset);
    let mut depth = 0i32;
    for (i, c) in content[..end].char_indices().rev() {
        if c == close && i != offset {
            depth += 1;
        } else if c == open {
            if depth == 0 {
                return Some(i);
            }
            depth -= 1;
        }
    }
    None
}

fn matching_close(content: &str, open_pos: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in chars_from(content, open_pos) {
        depth += (c == open) as i32 - (c == close) as i32;
        if depth == 0 {
            return Some(i);
        }
    }
    None
}

pub fn doc_start(content: &str) -> usize {
    first_nonblank(content, 0)
}

pub fn nth_char_right(content: &str, offset: usize, n: usize) -> usize {
    let end = line_end_exclusive(content, offset);
    (0..n).fold(offset, |pos, _| {
        if pos >= end {
            pos
        } else {
            pos + char_len_at(content, pos)
        }
    })
}

pub fn nth_char_left(content: &str, offset: usize, n: usize) -> usize {
    let start = line_start(content, offset);
    (0..n).fold(offset, |pos, _| {
        if pos <= start {
            pos
        } else {
            prev_char_offset(content, pos)
        }
    })
}

pub fn line_down(content: &str, offset: usize, n: usize) -> usize {
    let mut start = line_start(content, offset);
    for _ in 0..n {
        match content[start..].find('\n') {
            Some(index) if start + index + 1 < content.len() => start += index + 1,
            _ => break,
        }
    }
    start
}

pub fn line_up(content: &str, offset: usize, n: usize) -> usize {
    (0..n).fold(line_start(content, offset), |start, _| {
        if start == 0 {
            0
        } else {
            content[..start - 1].rfind('\n').map_or(0, |i| i + 1)
        }
    })
}

pub fn doc_end(content: &str) -> usize {
    let trimmed = content.trim_end_matches('\n');
    let last_line = line_start(content, trimmed.len());
    first_nonblank(content, last_line)
}

pub fn goto_line(content: &str, line: usize) -> usize {
    let mut start = 0;
    for _ in 1..line.max(1) {
        match content[start..].find('\n') {
            Some(index) if start + index + 1 < content.len() => start += index + 1,
            _ => break,
        }
    }
    first_nonblank(content, start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_forward_word_and_punct_classes() {
        // foo.bar baz -> stops at '.', then 'bar', then 'baz'
        let text = "foo.bar baz";
        assert_eq!(word_forward(text, 0, false), 3); // '.'
        assert_eq!(word_forward(text, 3, false), 4); // 'bar'
        assert_eq!(word_forward(text, 4, false), 8); // 'baz'
                                                     // WORD mode skips the punctuation as part of the word.
        assert_eq!(word_forward(text, 0, true), 8); // 'baz'
    }

    #[test]
    fn word_forward_stops_on_empty_line() {
        let text = "ab\n\ncd";
        assert_eq!(word_forward(text, 0, false), 3); // the empty line
        assert_eq!(word_forward(text, 3, false), 4); // 'cd'
    }

    #[test]
    fn word_forward_over_emoji() {
        // "a😀 b": a=0, emoji=1..5, space=5, b=6. The emoji is its own
        // (punctuation) word; stepping past it must land on the right byte.
        let text = "a😀 b";
        assert_eq!(word_forward(text, 0, false), 1); // onto the emoji
        assert_eq!(word_forward(text, 1, false), 6); // past the 4-byte emoji, onto 'b'
    }

    #[test]
    fn word_backward_basics() {
        let text = "foo bar";
        assert_eq!(word_backward(text, 7, false), 4); // start of 'bar'
        assert_eq!(word_backward(text, 4, false), 0); // start of 'foo'
        assert_eq!(word_backward(text, 0, false), 0);
    }

    #[test]
    fn word_end_basics() {
        let text = "foo bar";
        assert_eq!(word_end(text, 0, false), 2); // 'o' end of foo
        assert_eq!(word_end(text, 2, false), 6); // 'r' end of bar
    }

    #[test]
    fn line_motions() {
        let text = "  ab\ncd\n";
        assert_eq!(line_start(text, 3), 0);
        assert_eq!(first_nonblank(text, 3), 2); // 'a'
        assert_eq!(line_end(text, 0), 3); // 'b'
        assert_eq!(line_start(text, 6), 5);
        assert_eq!(line_end(text, 6), 6); // 'd'
    }

    #[test]
    fn line_end_on_empty_line() {
        let text = "a\n\nb";
        assert_eq!(line_end(text, 2), 2); // stays on the empty line
        assert_eq!(first_nonblank(text, 2), 2);
    }

    #[test]
    fn find_char_forward_and_till() {
        let text = "abcxdef";
        assert_eq!(find_char(text, 0, 'x', true, false), Some(3));
        assert_eq!(find_char(text, 0, 'x', true, true), Some(2)); // 't' stops before
        assert_eq!(find_char(text, 0, 'z', true, false), None);
        // does not cross the newline
        assert_eq!(find_char("ab\nxc", 0, 'x', true, false), None);
    }

    #[test]
    fn find_char_backward_and_till() {
        let text = "abcxdef";
        assert_eq!(find_char(text, 6, 'x', false, false), Some(3));
        assert_eq!(find_char(text, 6, 'x', false, true), Some(4)); // 'T' stops after
        assert_eq!(find_char(text, 6, 'z', false, false), None);
    }

    #[test]
    fn paragraph_motions() {
        let text = "a\n\nb\n\nc";
        assert_eq!(paragraph_forward(text, 0), 2); // empty line after 'a'
        assert_eq!(paragraph_forward(text, 3), 5); // empty line after 'b'
        assert_eq!(paragraph_forward(text, 6), text.len());
        assert_eq!(paragraph_backward(text, 6), 5);
        assert_eq!(paragraph_backward(text, 3), 2);
        assert_eq!(paragraph_backward(text, 0), 0);
    }

    #[test]
    fn matching_pair_all_kinds() {
        assert_eq!(matching_pair("(a[b]c)", 0), Some(6));
        assert_eq!(matching_pair("(a[b]c)", 6), Some(0));
        assert_eq!(matching_pair("(a[b]c)", 2), Some(4)); // '[' -> ']'
        assert_eq!(matching_pair("{ }", 0), Some(2));
        // finds the first bracket at or after the cursor on the line
        assert_eq!(matching_pair("xy(z)", 0), Some(4));
        assert_eq!(matching_pair("no brackets", 0), None);
    }

    #[test]
    fn doc_motions() {
        let text = "  first\nmid\n  last\n";
        assert_eq!(doc_start(text), 2); // 'f'
        assert_eq!(doc_end(text), 14); // 'l' of "last"
    }

    #[test]
    fn text_object_quotes() {
        let text = "say \"hello world\" now";
        let inner = text_object(text, 7, '"', false).unwrap();
        assert_eq!(&text[inner], "hello world");
        let around = text_object(text, 7, '"', true).unwrap();
        assert_eq!(&text[around], "\"hello world\"");
        // works from before the quotes too (vim searches forward on the line)
        let inner = text_object(text, 0, '"', false).unwrap();
        assert_eq!(&text[inner], "hello world");
    }

    #[test]
    fn text_object_nested_parens() {
        let text = "a(b(c)d)e";
        assert_eq!(&text[text_object(text, 4, '(', false).unwrap()], "c");
        assert_eq!(&text[text_object(text, 2, '(', false).unwrap()], "b(c)d");
        assert_eq!(&text[text_object(text, 2, '(', true).unwrap()], "(b(c)d)");
    }

    #[test]
    fn text_object_inner_and_around_word() {
        let text = "foo bar baz";
        assert_eq!(&text[text_object(text, 5, 'w', false).unwrap()], "bar");
        assert_eq!(&text[text_object(text, 5, 'w', true).unwrap()], "bar ");
    }

    #[test]
    fn text_object_missing_pair_is_none() {
        assert!(text_object("no quotes here", 0, '"', false).is_none());
    }

    #[test]
    fn goto_line_clamps() {
        let text = "one\ntwo\nthree\n";
        assert_eq!(goto_line(text, 1), 0); // 'one'
        assert_eq!(goto_line(text, 2), 4); // 'two'
        assert_eq!(goto_line(text, 3), 8); // 'three'
        assert_eq!(goto_line(text, 99), 8); // clamps to the last line
    }
}
