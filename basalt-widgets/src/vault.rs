use std::marker::PhantomData;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, Clear, List, ListItem, StatefulWidgetRef, Widget},
};

mod state;
pub use state::VaultSelectorState;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VaultSelector<'a> {
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> VaultSelector<'a> {
    pub fn new() -> Self {
        Self {
            _lifetime: PhantomData,
        }
    }
}

impl<'a> StatefulWidgetRef for VaultSelector<'a> {
    type State = VaultSelectorState<'a>;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let render_area = if state.is_modal {
            let modal_area = modal_area(area);
            Widget::render(Clear, modal_area, buf);
            modal_area
        } else {
            area
        };

        let items: Vec<ListItem> = state
            .items
            .iter()
            .map(|item| {
                if item.open {
                    ListItem::new(format!("◆ {}", item.name))
                } else {
                    ListItem::new(format!("  {}", item.name))
                }
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::bordered()
                    .black()
                    .title(" Vaults ")
                    .title_style(Style::default().italic().bold())
                    .border_type(BorderType::Rounded),
            )
            .fg(Color::default())
            .highlight_style(Style::new().reversed().dark_gray())
            .highlight_symbol(" ");

        StatefulWidgetRef::render_ref(&list, render_area, buf, &mut state.list_state);
    }
}

fn modal_area(area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(50)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(60)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
