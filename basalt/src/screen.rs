use crate::{main_view::MainView, start_view::StartView};

#[derive(Debug, Clone, PartialEq)]
pub enum Screen<'a> {
    Start(StartView<'a>),
    Main(MainView<'a>),
}

impl Default for Screen<'_> {
    fn default() -> Self {
        Screen::Start(StartView::default())
    }
}
