use textwrap::WordSeparator;

pub fn wrap_preserve_trailing(
    text: &str,
    max_width: usize,
    wrap_symbol_width: usize,
) -> Vec<String> {
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

        let wrap_symbol_width = match lines.is_empty() {
            false => wrap_symbol_width,
            true => 0,
        };

        // If the word (without dropping its trailing whitespace) fits, append it.
        if cur_width + width + wrap_symbol_width <= max_width {
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

    if !cur.is_empty() {
        lines.push(cur);
    }

    lines
}
