use textwrap::WordSeparator;

pub fn wrap_preserve_trailing(text: &str, first_width: usize, rest_width: usize) -> Vec<String> {
    let separator = WordSeparator::AsciiSpace;
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut cur_width = 0;

    for word in separator.find_words(text) {
        let word_with_trailing_whitespace = word.word.to_owned() + word.whitespace;
        let width = textwrap::core::display_width(&word_with_trailing_whitespace);

        // If the current line is empty, always put the word (even if too long).
        if cur.is_empty() {
            cur.push_str(&word_with_trailing_whitespace);
            cur_width = width;
            continue;
        }

        let max_width = match lines.is_empty() {
            true => first_width,
            false => rest_width,
        };

        // Trailing whitespace may run past the edge: it shows nothing there.
        if cur_width + textwrap::core::display_width(word.word) <= max_width {
            cur.push_str(&word_with_trailing_whitespace);
            cur_width += width;
        } else {
            // otherwise, end current line and start a new one.
            lines.push(cur);
            cur = String::new();
            cur.push_str(&word_with_trailing_whitespace);
            cur_width = width;
        }
    }

    // Always emit at least one line, even for empty input, so callers (e.g. an
    // empty list item `- `) still render a row instead of vanishing.
    if !cur.is_empty() || lines.is_empty() {
        lines.push(cur);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_fills_each_line_to_its_width() {
        assert_eq!(
            wrap_preserve_trailing("aaaa bbbb cccc dddd", 9, 4),
            ["aaaa bbbb ", "cccc ", "dddd"]
        );
    }

    #[test]
    fn test_wrap_keeps_an_overlong_word_whole() {
        assert_eq!(
            wrap_preserve_trailing("abcdefgh ij", 4, 4),
            ["abcdefgh ", "ij"]
        );
    }
}
