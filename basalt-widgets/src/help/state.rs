use ratatui::widgets::ScrollbarState;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct HelpModalState {
    pub scrollbar_state: ScrollbarState,
    pub scrollbar_position: usize,
    pub viewport_height: usize,
    pub text_lines: usize,
}

impl HelpModalState {
    pub fn new(text_lines: usize) -> Self {
        Self {
            text_lines,
            scrollbar_state: ScrollbarState::new(text_lines),
            ..Default::default()
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let scrollbar_position = self.scrollbar_position.saturating_sub(amount);
        self.scrollbar_state = self.scrollbar_state.position(scrollbar_position);
        self.scrollbar_position = scrollbar_position;
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let scrollbar_position = self
            .scrollbar_position
            .saturating_add(amount)
            .min(self.text_lines);

        self.scrollbar_state = self.scrollbar_state.position(scrollbar_position);
        self.scrollbar_position = scrollbar_position;
    }

    pub fn reset_scrollbar(&mut self) {
        self.scrollbar_state = ScrollbarState::default();
        self.scrollbar_position = 0;
    }
}
