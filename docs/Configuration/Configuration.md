[[Basalt]] can be customized using a TOML configuration file. The file does not exist by default — create it manually when you want to override the defaults.

## Configuration file location

[[Basalt]] looks for a configuration file in the following locations:

- **macOS and Linux**: `$HOME/.basalt.toml` or `$XDG_CONFIG_HOME/basalt/config.toml`
- **Windows**: `%USERPROFILE%\.basalt.toml` or `%APPDATA%\basalt\config.toml`

If configuration files exist in multiple locations, only the first one found is used, with the home directory taking precedence.

> [!WARNING]
>
> This behavior may change in future versions to merge all found configurations instead.

## Overriding defaults

Your configuration is **merged** with the defaults. You only need to define the key bindings you want to change — all other defaults remain active. If you bind a key that already exists in the defaults, your binding takes precedence.

For example, to change only the quit key:

```toml
[global]
key_bindings = [
  { key = "ctrl+q", command = "quit" },
]
```

This adds `Ctrl+Q` as a quit binding while keeping all other default global bindings (`?`, `Ctrl+G`, etc.) intact.

## Default configuration

The full default configuration is shown below. The default `exec:` and `spawn:` commands use macOS conventions (`vi`, `open`). On Linux, replace `open` with `xdg-open`; on Windows, use `start`. See [[Custom commands]] for details.

```toml
# Editor is experimental
experimental_editor = false

[global]
key_bindings = [
 { key = "q", command = "quit" },
 { key = "ctrl+g", command = "vault_selector_modal_toggle" },
 { key = "?", command = "help_modal_toggle" },
 { key = "ctrl+alt+e", command = "exec:vi %note_path" },
 { key = "ctrl+alt+o", command = "spawn:open obsidian://open?vault=%vault&file=%note" },
]

[splash]
key_bindings = [
 { key = "k", command = "splash_up" },
 { key = "j", command = "splash_down" },
 { key = "up", command = "splash_up" },
 { key = "down", command = "splash_down" },
 { key = "enter", command = "splash_open" },
]

[explorer]
key_bindings = [
 { key = "k", command = "explorer_up" },
 { key = "j", command = "explorer_down" },
 { key = "up", command = "explorer_up" },
 { key = "down", command = "explorer_down" },
 { key = "t", command = "explorer_toggle" },
 { key = "h", command = "explorer_hide_pane" },
 { key = "l", command = "explorer_expand_pane" },
 { key = "left", command = "explorer_hide_pane" },
 { key = "right", command = "explorer_expand_pane" },
 { key = "s", command = "explorer_sort" },
 { key = "r", command = "explorer_toggle_input_rename" },
 { key = "tab", command = "explorer_switch_pane_next" },
 { key = "shift+backtab", command = "explorer_switch_pane_previous" },
 { key = "enter", command = "explorer_open" },
 { key = "ctrl+b", command = "explorer_toggle" },
 { key = "ctrl+u", command = "explorer_scroll_up_half_page" },
 { key = "ctrl+d", command = "explorer_scroll_down_half_page" },
 { key = "ctrl+o", command = "explorer_toggle_outline" },
]

[outline]
key_bindings = [
 { key = "k", command = "outline_up" },
 { key = "j", command = "outline_down" },
 { key = "up", command = "outline_up" },
 { key = "down", command = "outline_down" },
 { key = "ctrl+o", command = "outline_toggle" },
 { key = "ctrl+b", command = "outline_toggle_explorer" },
 { key = "t", command = "outline_toggle_explorer" },
 { key = "tab", command = "outline_switch_pane_next" },
 { key = "shift+backtab", command = "outline_switch_pane_previous" },
 { key = "enter", command = "outline_expand" },
 { key = "g", command = "outline_select" },
]

[note_editor]
key_bindings = [
 { key = "k", command = "note_editor_cursor_up" },
 { key = "j", command = "note_editor_cursor_down" },
 { key = "up", command = "note_editor_cursor_up" },
 { key = "down", command = "note_editor_cursor_down" },
 { key = "t", command = "note_editor_toggle_explorer" },
 { key = "tab", command = "note_editor_switch_pane_next" },
 { key = "shift+backtab", command = "note_editor_switch_pane_previous" },
 { key = "ctrl+b", command = "note_editor_toggle_explorer" },
 { key = "ctrl+u", command = "note_editor_scroll_up_half_page" },
 { key = "ctrl+d", command = "note_editor_scroll_down_half_page" },
 { key = "ctrl+o", command = "note_editor_toggle_outline" },

 # Experimental editor
 { key = "i", command = "note_editor_experimental_set_edit_view" },
 { key = "ctrl+e", command = "note_editor_experimental_toggle_view" },
 { key = "shift+r", command = "note_editor_experimental_set_read_view" },
 { key = "ctrl+x", command = "note_editor_experimental_save" },
 { key = "esc", command = "note_editor_experimental_exit" },
 { key = "h", command = "note_editor_experimental_cursor_left" },
 { key = "l", command = "note_editor_experimental_cursor_right" },
 { key = "left", command = "note_editor_experimental_cursor_left" },
 { key = "right", command = "note_editor_experimental_cursor_right" },
 { key = "alt+f", command = "note_editor_experimental_cursor_word_forward" },
 { key = "alt+b", command = "note_editor_experimental_cursor_word_backward" },
]

[input_modal]
key_bindings = [
 { key = "esc", command = "input_modal_cancel" },
 { key = "enter", command = "input_modal_accept" },
 { key = "i", command = "input_modal_edit_mode" },
 { key = "h", command = "input_modal_left" },
 { key = "l", command = "input_modal_right" },
 { key = "left", command = "input_modal_left" },
 { key = "right", command = "input_modal_right" },
 { key = "alt+f", command = "input_modal_word_forward" },
 { key = "alt+b", command = "input_modal_word_backward" },
]

[help_modal]
key_bindings = [
 { key = "esc", command = "help_modal_close" },
 { key = "k", command = "help_modal_scroll_up_one" },
 { key = "j", command = "help_modal_scroll_down_one" },
 { key = "up", command = "help_modal_scroll_up_one" },
 { key = "down", command = "help_modal_scroll_down_one" },
 { key = "ctrl+u", command = "help_modal_scroll_up_half_page" },
 { key = "ctrl+d", command = "help_modal_scroll_down_half_page" },
]

[vault_selector_modal]
key_bindings = [
 { key = "k", command = "vault_selector_modal_up" },
 { key = "j", command = "vault_selector_modal_down" },
 { key = "up", command = "vault_selector_modal_up" },
 { key = "down", command = "vault_selector_modal_down" },
 { key = "enter", command = "vault_selector_modal_open" },
 { key = "esc", command = "vault_selector_modal_close" },
]
```
