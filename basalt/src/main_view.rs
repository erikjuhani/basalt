use basalt_core::obsidian::Vault;
use basalt_widgets::markdown::MarkdownViewState;
use ki::{
    explorer::{state::ExplorerState, Explorer},
    fs::SortablePath,
};
use ratatui::layout::Size;
use std::collections::BTreeSet;
use std::io::Result;

use crate::mode::Mode;

#[derive(Debug, Clone, PartialEq)]
pub struct MainView<'a> {
    pub explorer: Explorer<'a, SortablePath>,
    pub explorer_state: ExplorerState<SortablePath>,
    pub markdown_view_state: MarkdownViewState,
    pub size: Size,
    pub mode: Mode,
}

impl<'a> MainView<'a> {
    pub fn new(vault: &'a Vault, entries: BTreeSet<SortablePath>, size: Size) -> Result<Self> {
        let mut explorer = Explorer::new(&vault.name, &vault.path)?;

        explorer.add_entries(entries)?;

        let mut explorer_state = ExplorerState::default();
        explorer_state.open = true;

        let first_level_entries = vault.load_first_level().unwrap();

        explorer_state.last_identifiers = first_level_entries
            .clone()
            .into_iter()
            .map(|entry| vec![entry])
            .collect();

        explorer_state.last_biggest_index = explorer_state.last_identifiers.len().saturating_sub(1);

        // Auto-select the first entry if available
        if let Some(first_entry) = explorer_state.last_identifiers.first() {
            explorer_state.select(first_entry.clone());
        }

        explorer_state.scroll_selected_into_view();

        Ok(Self {
            explorer,
            explorer_state,
            markdown_view_state: MarkdownViewState::default(),
            size,
            mode: Mode::Select, // Default to Select mode for explorer interaction
        })
    }
}
