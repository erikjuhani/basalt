/// A wrapper type representing the number of characters in a string. **All** characters are
/// counted for.
///
/// Character count can be created from an `usize` directly or computed from a `&str`.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct CharCount(usize);

impl From<usize> for CharCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<&str> for CharCount {
    fn from(value: &str) -> Self {
        value.chars().count().into()
    }
}

impl From<CharCount> for usize {
    fn from(value: CharCount) -> Self {
        value.0
    }
}

/// A wrapper type representing the number of words in a string.
///
/// Can be created from a `usize` directly or computed from a `&str` by counting the number of
/// whitespace-separated words, after removing special markdown characters.
///
/// markdown characters: * _ ` < > ? ! [ ] ( ) = ~ # +
#[derive(Default, Clone, Debug, PartialEq)]
pub struct WordCount(usize);

impl From<WordCount> for usize {
    fn from(value: WordCount) -> Self {
        value.0
    }
}

impl From<usize> for WordCount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<&str> for WordCount {
    fn from(value: &str) -> Self {
        let special_symbols = [
            '*', '_', '`', '<', '>', '?', '!', '[', ']', '(', ')', '=', '~', '#', '+',
        ];

        value
            .replace(special_symbols, "")
            .split_whitespace()
            .count()
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use similar_asserts::assert_eq;

    #[test]
    fn test_text_counts() {
        let tests = [
            (
                indoc! {r#"# Heading 1

                ## Heading 2

                ### Heading 3

                #### Heading 4

                ##### Heading 5

                ###### Heading 6"#},
                (WordCount(12), CharCount(91)),
            ),
            (
                indoc! { r#"## Tasks

                - [ ] Task

                - [x] Completed task

                - [?] Completed task"#},
                (WordCount(10), CharCount(64)),
            ),
            (
                indoc! {r#"## Quotes

                You _can_ quote text by adding a `>` symbols before the text.

                > Human beings face ever more complex and urgent problems, and their effectiveness in dealing with these problems is a matter that is critical to the stability and continued progress of society.
                >
                >- Doug Engelbart, 1961"#},
                (WordCount(47), CharCount(294)),
            ),
        ];

        tests.into_iter().for_each(|(input, expected)| {
            assert_eq!(
                (WordCount::from(input), CharCount::from(input)),
                expected,
                "With input {input}"
            )
        });
    }
}
