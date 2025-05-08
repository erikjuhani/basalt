use ratatui::widgets::ScrollbarState;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Scrollbar {
    pub state: ScrollbarState,
    pub position: usize,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MarkdownViewState {
    pub(crate) text: String,
    pub(crate) scrollbar: Scrollbar,
}

impl MarkdownViewState {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    pub fn get_lines(&self) -> Vec<&str> {
        self.text.lines().collect()
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let new_position = self.scrollbar.position.saturating_sub(amount);
        let new_state = self.scrollbar.state.position(new_position);

        self.scrollbar.state = new_state;
        self.scrollbar.position = new_position;
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let new_position = self.scrollbar.position.saturating_add(amount);
        let new_state = self.scrollbar.state.position(new_position);

        self.scrollbar.state = new_state;
        self.scrollbar.position = new_position;
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    pub fn reset_scrollbar(&mut self) {
        self.scrollbar = Scrollbar::default();
        self.scrollbar.position = 0;
    }
}
