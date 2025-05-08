use crate::app::ScrollAmount;
use ratatui::layout::Size;

#[derive(Debug, PartialEq)]
pub enum Action {
    Select,
    Next,
    Prev,
    Insert,
    Resize(Size),
    ScrollUp(ScrollAmount),
    ScrollDown(ScrollAmount),
    ToggleEntry,
    ToggleMode,
    ToggleHelp,
    ToggleVaultSelector,
    Quit,
}
