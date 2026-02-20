use ratatui::{
    crossterm::{
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    DefaultTerminal,
};
use serde::{Deserialize, Deserializer};
use std::{io::stdout, process};

use crate::{
    app::{Message, ScrollAmount},
    explorer, help_modal, input, note_editor, outline, splash_modal, vault_selector_modal,
};

trait ReplaceVar {
    fn replace_var(&self, variable: &str, content: &str) -> Self;
}

impl ReplaceVar for String {
    fn replace_var(&self, variable: &str, content: &str) -> Self {
        self.replace(variable, content)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Command {
    Quit,

    SplashUp,
    SplashDown,
    SplashOpen,

    ExplorerUp,
    ExplorerDown,
    ExplorerOpen,
    ExplorerSort,
    ExplorerToggle,
    ExplorerToggleInputFind,
    ExplorerToggleInputRename,
    ExplorerToggleOutline,
    ExplorerSwitchPaneNext,
    ExplorerSwitchPanePrevious,
    ExplorerHidePane,
    ExplorerExpandPane,
    ExplorerScrollUpOne,
    ExplorerScrollDownOne,
    ExplorerScrollUpHalfPage,
    ExplorerScrollDownHalfPage,

    OutlineUp,
    OutlineDown,
    OutlineSelect,
    OutlineExpand,
    OutlineToggle,
    OutlineToggleExplorer,
    OutlineSwitchPaneNext,
    OutlineSwitchPanePrevious,

    HelpModalScrollUpOne,
    HelpModalScrollDownOne,
    HelpModalScrollUpHalfPage,
    HelpModalScrollDownHalfPage,
    HelpModalToggle,
    HelpModalClose,

    NoteEditorScrollUpOne,
    NoteEditorScrollDownOne,
    NoteEditorScrollUpHalfPage,
    NoteEditorScrollDownHalfPage,
    NoteEditorSwitchPaneNext,
    NoteEditorSwitchPanePrevious,
    NoteEditorToggleExplorer,
    NoteEditorToggleOutline,
    NoteEditorCursorUp,
    NoteEditorCursorDown,

    NoteEditorExperimentalCursorWordForward,
    NoteEditorExperimentalCursorWordBackward,
    NoteEditorExperimentalToggleView,
    NoteEditorExperimentalSetEditView,
    NoteEditorExperimentalSetReadView,
    NoteEditorExperimentalSave,
    NoteEditorExperimentalExit,
    NoteEditorExperimentalCursorLeft,
    NoteEditorExperimentalCursorRight,

    VaultSelectorModalUp,
    VaultSelectorModalDown,
    VaultSelectorModalClose,
    VaultSelectorModalOpen,
    VaultSelectorModalToggle,

    InputModalWordForward,
    InputModalWordBackward,
    InputModalLeft,
    InputModalRight,
    InputModalCancel,
    InputModalAccept,
    InputModalEditMode,

    Exec(String),
    Spawn(String),
}

fn str_to_command(s: &str) -> Option<Command> {
    match s {
        "quit" => Some(Command::Quit),

        "splash_up" => Some(Command::SplashUp),
        "splash_down" => Some(Command::SplashDown),
        "splash_open" => Some(Command::SplashOpen),

        "explorer_up" => Some(Command::ExplorerUp),
        "explorer_down" => Some(Command::ExplorerDown),
        "explorer_open" => Some(Command::ExplorerOpen),
        "explorer_sort" => Some(Command::ExplorerSort),
        "explorer_toggle" => Some(Command::ExplorerToggle),
        "explorer_toggle_input_find" => Some(Command::ExplorerToggleInputFind),
        "explorer_toggle_outline" => Some(Command::ExplorerToggleOutline),
        "explorer_toggle_input_rename" => Some(Command::ExplorerToggleInputRename),
        "explorer_switch_pane_next" => Some(Command::ExplorerSwitchPaneNext),
        "explorer_hide_pane" => Some(Command::ExplorerHidePane),
        "explorer_expand_pane" => Some(Command::ExplorerExpandPane),
        "explorer_switch_pane_previous" => Some(Command::ExplorerSwitchPanePrevious),
        "explorer_scroll_up_one" => Some(Command::ExplorerScrollUpOne),
        "explorer_scroll_down_one" => Some(Command::ExplorerScrollDownOne),
        "explorer_scroll_up_half_page" => Some(Command::ExplorerScrollUpHalfPage),
        "explorer_scroll_down_half_page" => Some(Command::ExplorerScrollDownHalfPage),

        "input_modal_word_forward" => Some(Command::InputModalWordForward),
        "input_modal_word_backward" => Some(Command::InputModalWordBackward),
        "input_modal_left" => Some(Command::InputModalLeft),
        "input_modal_right" => Some(Command::InputModalRight),
        "input_modal_cancel" => Some(Command::InputModalCancel),
        "input_modal_accept" => Some(Command::InputModalAccept),
        "input_modal_edit_mode" => Some(Command::InputModalEditMode),

        "outline_up" => Some(Command::OutlineUp),
        "outline_down" => Some(Command::OutlineDown),
        "outline_select" => Some(Command::OutlineSelect),
        "outline_expand" => Some(Command::OutlineExpand),
        "outline_toggle" => Some(Command::OutlineToggle),
        "outline_toggle_explorer" => Some(Command::OutlineToggleExplorer),
        "outline_switch_pane_next" => Some(Command::OutlineSwitchPaneNext),
        "outline_switch_pane_previous" => Some(Command::OutlineSwitchPanePrevious),

        "help_modal_scroll_up_one" => Some(Command::HelpModalScrollUpOne),
        "help_modal_scroll_down_one" => Some(Command::HelpModalScrollDownOne),
        "help_modal_scroll_up_half_page" => Some(Command::HelpModalScrollUpHalfPage),
        "help_modal_scroll_down_half_page" => Some(Command::HelpModalScrollDownHalfPage),
        "help_modal_toggle" => Some(Command::HelpModalToggle),
        "help_modal_close" => Some(Command::HelpModalClose),

        "note_editor_scroll_up_one" => Some(Command::NoteEditorScrollUpOne),
        "note_editor_scroll_down_one" => Some(Command::NoteEditorScrollDownOne),
        "note_editor_scroll_up_half_page" => Some(Command::NoteEditorScrollUpHalfPage),
        "note_editor_scroll_down_half_page" => Some(Command::NoteEditorScrollDownHalfPage),
        "note_editor_switch_pane_next" => Some(Command::NoteEditorSwitchPaneNext),
        "note_editor_switch_pane_previous" => Some(Command::NoteEditorSwitchPanePrevious),
        "note_editor_toggle_explorer" => Some(Command::NoteEditorToggleExplorer),
        "note_editor_toggle_outline" => Some(Command::NoteEditorToggleOutline),
        "note_editor_cursor_up" => Some(Command::NoteEditorCursorUp),
        "note_editor_cursor_down" => Some(Command::NoteEditorCursorDown),

        "note_editor_experimental_cursor_word_forward" => {
            Some(Command::NoteEditorExperimentalCursorWordForward)
        }
        "note_editor_experimental_cursor_word_backward" => {
            Some(Command::NoteEditorExperimentalCursorWordBackward)
        }
        "note_editor_experimental_set_edit_view" => {
            Some(Command::NoteEditorExperimentalSetEditView)
        }
        "note_editor_experimental_toggle_view" => Some(Command::NoteEditorExperimentalToggleView),
        "note_editor_experimental_set_read_view" => {
            Some(Command::NoteEditorExperimentalSetReadView)
        }
        "note_editor_experimental_save" => Some(Command::NoteEditorExperimentalSave),
        "note_editor_experimental_exit" => Some(Command::NoteEditorExperimentalExit),
        "note_editor_experimental_cursor_left" => Some(Command::NoteEditorExperimentalCursorLeft),
        "note_editor_experimental_cursor_right" => Some(Command::NoteEditorExperimentalCursorRight),

        "vault_selector_modal_up" => Some(Command::VaultSelectorModalUp),
        "vault_selector_modal_down" => Some(Command::VaultSelectorModalDown),
        "vault_selector_modal_close" => Some(Command::VaultSelectorModalClose),
        "vault_selector_modal_open" => Some(Command::VaultSelectorModalOpen),
        "vault_selector_modal_toggle" => Some(Command::VaultSelectorModalToggle),

        // TODO: Remove deprecations in the next major version
        // Deprecated
        "note_editor_experimental_set_edit_mode" => {
            Some(Command::NoteEditorExperimentalSetEditView)
        }
        // Deprecated
        "note_editor_experimental_set_read_mode" => {
            Some(Command::NoteEditorExperimentalSetReadView)
        }
        // Deprecated
        "note_editor_experimental_exit_mode" => Some(Command::NoteEditorExperimentalExit),
        _ => None,
    }
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if let Some(command) = s
            .strip_prefix("exec:")
            .map(|command| Command::Exec(command.to_string()))
            .or(s
                .strip_prefix("spawn:")
                .map(|command| Command::Spawn(command.to_string())))
        {
            return Ok(command);
        }

        str_to_command(&s).ok_or(serde::de::Error::custom(format!(
            "{s} is not a valid command"
        )))
    }
}

impl From<Command> for Message<'_> {
    fn from(value: Command) -> Self {
        match value {
            Command::Quit => Message::Quit,

            Command::SplashUp => Message::Splash(splash_modal::Message::Up),
            Command::SplashDown => Message::Splash(splash_modal::Message::Down),
            Command::SplashOpen => Message::Splash(splash_modal::Message::Open),

            Command::ExplorerUp => Message::Explorer(explorer::Message::Up),
            Command::ExplorerDown => Message::Explorer(explorer::Message::Down),
            Command::ExplorerOpen => Message::Explorer(explorer::Message::Open),
            Command::ExplorerSort => Message::Explorer(explorer::Message::Sort),
            Command::ExplorerToggle => Message::Explorer(explorer::Message::Toggle),
            Command::ExplorerToggleInputFind => {
                Message::Explorer(explorer::Message::ToggleInputFind)
            }
            Command::ExplorerToggleOutline => Message::Explorer(explorer::Message::ToggleOutline),
            Command::ExplorerToggleInputRename => {
                Message::Explorer(explorer::Message::ToggleInputRename)
            }
            Command::ExplorerHidePane => Message::Explorer(explorer::Message::HidePane),
            Command::ExplorerExpandPane => Message::Explorer(explorer::Message::ExpandPane),
            Command::ExplorerSwitchPaneNext => Message::Explorer(explorer::Message::SwitchPaneNext),
            Command::ExplorerSwitchPanePrevious => {
                Message::Explorer(explorer::Message::SwitchPanePrevious)
            }
            Command::ExplorerScrollUpOne => {
                Message::Explorer(explorer::Message::ScrollUp(ScrollAmount::One))
            }
            Command::ExplorerScrollDownOne => {
                Message::Explorer(explorer::Message::ScrollDown(ScrollAmount::One))
            }
            Command::ExplorerScrollUpHalfPage => {
                Message::Explorer(explorer::Message::ScrollUp(ScrollAmount::HalfPage))
            }
            Command::ExplorerScrollDownHalfPage => {
                Message::Explorer(explorer::Message::ScrollDown(ScrollAmount::HalfPage))
            }

            Command::InputModalEditMode => Message::Input(input::Message::EditMode),
            Command::InputModalAccept => Message::Input(input::Message::Accept),
            Command::InputModalCancel => Message::Input(input::Message::Cancel),
            Command::InputModalLeft => Message::Input(input::Message::CursorLeft),
            Command::InputModalRight => Message::Input(input::Message::CursorRight),
            Command::InputModalWordForward => Message::Input(input::Message::CursorWordForward),
            Command::InputModalWordBackward => Message::Input(input::Message::CursorWordBackward),

            Command::OutlineUp => Message::Outline(outline::Message::Up),
            Command::OutlineDown => Message::Outline(outline::Message::Down),
            Command::OutlineSelect => Message::Outline(outline::Message::Select),
            Command::OutlineExpand => Message::Outline(outline::Message::Expand),
            Command::OutlineToggle => Message::Outline(outline::Message::Toggle),
            Command::OutlineToggleExplorer => Message::Outline(outline::Message::ToggleExplorer),
            Command::OutlineSwitchPaneNext => Message::Outline(outline::Message::SwitchPaneNext),
            Command::OutlineSwitchPanePrevious => {
                Message::Outline(outline::Message::SwitchPanePrevious)
            }

            Command::HelpModalScrollUpOne => {
                Message::HelpModal(help_modal::Message::ScrollUp(ScrollAmount::One))
            }
            Command::HelpModalScrollDownOne => {
                Message::HelpModal(help_modal::Message::ScrollDown(ScrollAmount::One))
            }
            Command::HelpModalScrollUpHalfPage => {
                Message::HelpModal(help_modal::Message::ScrollUp(ScrollAmount::HalfPage))
            }
            Command::HelpModalScrollDownHalfPage => {
                Message::HelpModal(help_modal::Message::ScrollDown(ScrollAmount::HalfPage))
            }
            Command::HelpModalToggle => Message::HelpModal(help_modal::Message::Toggle),
            Command::HelpModalClose => Message::HelpModal(help_modal::Message::Close),

            Command::NoteEditorScrollUpOne => {
                Message::NoteEditor(note_editor::Message::ScrollUp(ScrollAmount::One))
            }
            Command::NoteEditorScrollDownOne => {
                Message::NoteEditor(note_editor::Message::ScrollDown(ScrollAmount::One))
            }
            Command::NoteEditorScrollUpHalfPage => {
                Message::NoteEditor(note_editor::Message::ScrollUp(ScrollAmount::HalfPage))
            }
            Command::NoteEditorScrollDownHalfPage => {
                Message::NoteEditor(note_editor::Message::ScrollDown(ScrollAmount::HalfPage))
            }
            Command::NoteEditorSwitchPaneNext => {
                Message::NoteEditor(note_editor::Message::SwitchPaneNext)
            }
            Command::NoteEditorSwitchPanePrevious => {
                Message::NoteEditor(note_editor::Message::SwitchPanePrevious)
            }
            Command::NoteEditorCursorUp => Message::NoteEditor(note_editor::Message::CursorUp),
            Command::NoteEditorCursorDown => Message::NoteEditor(note_editor::Message::CursorDown),
            Command::NoteEditorToggleExplorer => {
                Message::NoteEditor(note_editor::Message::ToggleExplorer)
            }
            Command::NoteEditorToggleOutline => {
                Message::NoteEditor(note_editor::Message::ToggleOutline)
            }

            // Experimental
            Command::NoteEditorExperimentalToggleView => {
                Message::NoteEditor(note_editor::Message::ToggleView)
            }
            Command::NoteEditorExperimentalSetEditView => {
                Message::NoteEditor(note_editor::Message::EditView)
            }
            Command::NoteEditorExperimentalSetReadView => {
                Message::NoteEditor(note_editor::Message::ReadView)
            }
            Command::NoteEditorExperimentalSave => Message::NoteEditor(note_editor::Message::Save),
            Command::NoteEditorExperimentalExit => Message::NoteEditor(note_editor::Message::Exit),
            Command::NoteEditorExperimentalCursorWordForward => {
                Message::NoteEditor(note_editor::Message::CursorWordForward)
            }
            Command::NoteEditorExperimentalCursorWordBackward => {
                Message::NoteEditor(note_editor::Message::CursorWordBackward)
            }
            Command::NoteEditorExperimentalCursorLeft => {
                Message::NoteEditor(note_editor::Message::CursorLeft)
            }
            Command::NoteEditorExperimentalCursorRight => {
                Message::NoteEditor(note_editor::Message::CursorRight)
            }

            Command::VaultSelectorModalClose => {
                Message::VaultSelectorModal(vault_selector_modal::Message::Close)
            }
            Command::VaultSelectorModalToggle => {
                Message::VaultSelectorModal(vault_selector_modal::Message::Toggle)
            }
            Command::VaultSelectorModalUp => {
                Message::VaultSelectorModal(vault_selector_modal::Message::Up)
            }
            Command::VaultSelectorModalDown => {
                Message::VaultSelectorModal(vault_selector_modal::Message::Down)
            }
            Command::VaultSelectorModalOpen => {
                Message::VaultSelectorModal(vault_selector_modal::Message::Select)
            }

            Command::Exec(command) => Message::Exec(command),
            Command::Spawn(command) => Message::Spawn(command),
        }
    }
}

pub fn run_command<'a>(
    command: String,
    vault_name: &str,
    note_name: &str,
    note_path: &str,
    mut callback: impl FnMut(&str, &[&str]) -> Option<Message<'a>>,
) -> Option<Message<'a>> {
    let expanded = command
        .replace_var("%vault", vault_name)
        // Order matters, otherwise all mentions of %note_path would be replaced with %note value
        .replace_var("%note_path", note_path)
        .replace_var("%note", note_name);

    let args = expanded.split_whitespace().collect::<Vec<_>>();

    match args.as_slice() {
        [command, args @ ..] => callback(command, args),
        [] => None,
    }
}

pub fn sync_command<'a>(
    terminal: &mut DefaultTerminal,
    command: String,
    vault_name: &str,
    note_name: &str,
    note_path: &str,
) -> Option<Message<'a>> {
    fn enter_alternate_screen(terminal: &mut DefaultTerminal) -> Result<(), std::io::Error> {
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        terminal.clear()
    }

    run_command(
        command,
        vault_name,
        note_name,
        note_path,
        |command, args| {
            // TODO:Error handling
            process::Command::new(command)
                .arg(args.join(" "))
                .status()
                .ok()?;
            enter_alternate_screen(terminal)
                .map(|_| Message::Explorer(explorer::Message::Open))
                .ok()
        },
    )
}

pub fn spawn_command<'a>(
    command: String,
    vault_name: &str,
    note_name: &str,
    note_path: &str,
) -> Option<Message<'a>> {
    run_command(
        command,
        vault_name,
        note_name,
        note_path,
        |command, args| {
            // TODO:Error handling
            _ = process::Command::new(command)
                .arg(args.join(" "))
                .spawn()
                .ok();
            None
        },
    )
}
