#[derive(Default, Clone, PartialEq)]
pub struct StatusBarState<'a> {
    pub(super) mode: &'a str,
    pub(super) meta: Option<&'a str>,
    pub(super) word_count: usize,
    pub(super) char_count: usize,
}

impl<'a> StatusBarState<'a> {
    pub fn new(mode: &'a str, meta: Option<&'a str>, word_count: usize, char_count: usize) -> Self {
        Self {
            mode,
            meta,
            word_count,
            char_count,
        }
    }
}
