[[Basalt]] can be customized using a TOML configuration file. The file does not exist by default. Create it manually when you want to override the defaults.

Press `?` at any time to see the active keymap for the current pane:

![[help-modal.avif]]

## Configuration file location

[[Basalt]] looks for a configuration file in the following locations:

- **macOS and Linux**: `$HOME/.basalt.toml` or `$XDG_CONFIG_HOME/basalt/config.toml`
- **Windows**: `%USERPROFILE%\.basalt.toml` or `%APPDATA%\basalt\config.toml`

If configuration files exist in multiple locations, only the first one found is used, with the home directory taking precedence.

> [!WARNING]
>
> This behavior may change in future versions to merge all found configurations instead.

## Overriding defaults

Your configuration is **merged** with the defaults. You only need to define the key bindings you want to change. All other defaults remain active. If you bind a key that already exists in the defaults, your binding takes precedence.

For example, to change only the quit key:

```toml
[global]
key_bindings = [
  { key = "ctrl+q", command = "quit" },
]
```

This adds `Ctrl+Q` as a quit binding while keeping all other default global bindings (`?`, `<leader>v`, etc.) intact.

## Leader key

Bindings can be written against a `<leader>` prefix instead of a fixed key. The
leader defaults to `<space>` and is set at the top level:

```toml
leader = ","
```

See [[Key mappings]] for the binding syntax.

## Note editor

The `[note_editor]` table holds the editor settings and its key bindings:

- `experimental`: enables the experimental editor. It is off by default.
- `vim_mode`: enables the vim keybinding preset (see [Vim mode](#vim-mode)).
- `default_mode`: the view a note opens in, `read` (default) or `edit`.

```toml
[note_editor]
experimental = true
default_mode = "edit"
```

`default_mode = "edit"` needs the experimental editor. When `experimental = false`, the note opens in READ view even if you set `edit`.

## Tabs

[[Basalt]] can sync tabs with the vault's `.obsidian/workspace.json`. Tabs themselves stay available either way; the `[tabs]` table's `sync` setting only controls syncing with Obsidian:

- `"off"`: don't touch `workspace.json` at all.
- `"read"` (default): read open tabs from it.
- `"write"`: also write tab changes back to it.

```toml
[tabs]
sync = "write"
```

> [!WARNING]
>
> `.obsidian/workspace.json` is a file Obsidian also owns. A bug in the write path could corrupt it, so `"write"` is experimental.

## Vim mode

Setting `vim_mode = true` in the `[note_editor]` table enables a built-in keybinding preset modelled after vim. For each section it defines, the vim preset **replaces** the default bindings entirely rather than merging with them. Your own config is still merged on top, so individual bindings can still be overridden.

```toml
[note_editor]
vim_mode = true
```

In the note editor, vim mode introduces Normal and Insert sub-modes within EDIT. Press `i` to enter Insert mode for typing. Press `Esc` to return to Normal mode for navigation. Press `Esc` again to exit back to READ.

### Motions

Normal mode carries the motions you expect from vim. See [[Key mappings]] for the command name behind each key.

| Key             | Motion                                        |
| --------------- | --------------------------------------------- |
| `h` / `l`       | Move left and right                           |
| `w` / `b`       | Move forward and backward by word             |
| `e`             | Move to the end of the word                   |
| `W` / `B` / `E` | The same three by whitespace-delimited word   |
| `0` / `^` / `$` | Line start, first non-blank, line end         |
| `gg` / `G`      | Start and end of the note                     |
| `gk` / `gj`     | Up and down one visible row of a wrapped line |
| `{` / `}`       | Previous and next paragraph                   |
| `%`             | Jump to the matching bracket                  |
| `f` / `F`       | Jump to the next typed character              |
| `t` / `T`       | The same, stopping short of the character     |
| `;` / `,`       | Repeat the last find, forward and in reverse  |
| `gx`            | Follow the link under the cursor              |

![[vim-motions.avif]]

### Operators and edits

| Key        | Action                                        |
| ---------- | --------------------------------------------- |
| `d` / `c`  | Delete and change; each takes a motion        |
| `x`        | Delete the character under the cursor         |
| `D` / `C`  | Delete and change to the end of the line      |
| `s` / `r`  | Substitute and replace a character            |
| `p` / `P`  | Paste after and before the cursor             |
| `a`        | Enter insert mode after the cursor            |
| `u`        | Undo the last change                          |
| `Ctrl+R`   | Redo the undone change                        |

### Visual selection and yank

From Normal mode you can select text and yank it to the system clipboard:

| Key   | Command                          |
| ----- | -------------------------------- |
| `v`   | Start a charwise selection       |
| `V`   | Start a linewise selection       |
| `y`   | Yank the selection to clipboard  |
| `Esc` | Cancel the selection             |

Motions extend the selection, a short flash marks the yanked range and the copy uses your platform clipboard utility with an OSC 52 fallback for SSH and tmux.

![[visual-selection.avif]]

### Other panes

| Pane        | Key        | Command                                 |
| ----------- | ---------- | --------------------------------------- |
| Explorer    | `gg` / `G` | Jump to the first and last item         |
| Input modal | `w` / `b`  | Move the cursor forward and back a word |

The full vim preset is defined in `vim.toml`. Sections not defined in the preset keep their default bindings.

## Symbols

The `[symbols]` table controls the visual glyphs used across the interface. Basalt ships with three presets: `unicode` (default), `ascii` and `nerd-font`. Individual symbols can be overridden on top of any preset. See [[Symbols]] for the full reference.

```toml
[symbols]
preset = "unicode"
```

## Themes

The `theme` key sets the interface colour scheme. Basalt ships with themes like `gruvbox-dark`, `everforest-dark`, `catppuccin-mocha` and its own `causeway-dark`, and you can preview them live with `<leader>t` or drop your own into the themes directory. See [[Themes]] for the full role reference and how to create one.

```toml
theme = "causeway-dark"
```

## Default configuration

The full default configuration is shown below, mirroring `basalt/config.toml` in the repository. The default `exec:` and `spawn:` commands use macOS conventions (`vi`, `open`). On Linux, replace `open` with `xdg-open`; on Windows, use `start`. See [[Custom commands]] for details.

```toml
wrap = true

# Line-number gutter shown while editing a note. "absolute" numbers each line,
# "relative" shows the distance from the cursor line (the cursor line keeps its
# absolute number), and "off" hides the gutter.
line_numbers = "absolute"

# The key that `<leader>` stands for in key bindings, e.g. `{ key =
# "<leader>f", command = "vault_selector_modal_toggle" }`. Only the leader set
# in the user config takes effect.
leader = "<space>"

# Colour theme. Built-in: "default", "causeway-dark", "causeway-light",
# "gruvbox-dark", "gruvbox-light", "everforest-dark", "everforest-light",
# "nord", "dracula", "catppuccin-latte", "catppuccin-frappe",
# "catppuccin-macchiato", "catppuccin-mocha", "minimal". Add your own by
# dropping a <name>.toml in $config/basalt/themes/ (see the bundled themes for
# the format); it then appears in the theme picker too. Themes colour text,
# backgrounds, headings, code and markdown, the status bar, and per-pane
# borders (including which edges draw, so a theme can keep only the dividers
# between panes).
theme = "default"

[symbols]
preset = "unicode"

[global]
key_bindings = [
 { key = "q", command = "quit" },
 { key = "?", command = "help_modal_toggle" },
 { key = "<leader>s", command = "search_toggle" },
 { key = "<leader>v", command = "vault_selector_modal_toggle" },
 { key = "<leader>d", command = "debug_log_toggle" },
 { key = "<leader>t", command = "theme_selector_modal_toggle" },
 { key = "ctrl+n", command = "tab_next" },
 { key = "ctrl+p", command = "tab_previous" },
 { key = "ctrl+w", command = "tab_close" },
 { key = "L", command = "tab_next" },
 { key = "H", command = "tab_previous" },
 { key = "]b", command = "tab_next" },
 { key = "[b", command = "tab_previous" },
 { key = "<leader>e", command = "exec:vi %note_path" },
 { key = "<leader>o", command = "spawn:open obsidian://open?vault=%vault&file=%note" },
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
 { key = "n", command = "explorer_new_untitled_note" },
 { key = "shift+n", command = "explorer_new_untitled_folder" },
 { key = "r", command = "explorer_toggle_input_rename" },
 { key = "tab", command = "explorer_switch_pane_next" },
 { key = "shift+backtab", command = "explorer_switch_pane_previous" },
 { key = "enter", command = "explorer_open" },
 { key = "ctrl+b", command = "explorer_toggle" },
 { key = "ctrl+u", command = "explorer_scroll_up_half_page" },
 { key = "ctrl+d", command = "explorer_scroll_down_half_page" },
 { key = "ctrl+o", command = "explorer_toggle_outline" },
 { key = "ctrl+shift+up", command = "explorer_scroll_to_top" },
 { key = "ctrl+shift+down", command = "explorer_scroll_to_bottom" },
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
 { key = "ctrl+shift+up", command = "explorer_scroll_to_top" },
 { key = "ctrl+shift+down", command = "explorer_scroll_to_bottom" },
]

[note_editor]
# The editor is experimental
experimental = false
vim_mode = false
# The view a note opens in: "read" or "edit". "edit" needs the experimental
# editor; without it the note opens in "read".
default_mode = "read"
key_bindings = [
 { key = "k", command = "note_editor_cursor_up" },
 { key = "j", command = "note_editor_cursor_down" },
 { key = "gk", command = "note_editor_cursor_screen_up" },
 { key = "gj", command = "note_editor_cursor_screen_down" },
 { key = "up", command = "note_editor_cursor_up" },
 { key = "down", command = "note_editor_cursor_down" },
 { key = "t", command = "note_editor_toggle_explorer" },
 { key = "tab", command = "note_editor_switch_pane_next" },
 { key = "shift+backtab", command = "note_editor_switch_pane_previous" },
 { key = "ctrl+b", command = "note_editor_toggle_explorer" },
 { key = "ctrl+u", command = "note_editor_scroll_up_half_page" },
 { key = "ctrl+d", command = "note_editor_scroll_down_half_page" },
 { key = "ctrl+o", command = "note_editor_toggle_outline" },
 { key = "ctrl+shift+up", command = "note_editor_scroll_to_top" },
 { key = "ctrl+shift+down", command = "note_editor_scroll_to_bottom" },
 { key = "enter", command = "note_editor_follow_link" },
 { key = "gd", command = "note_editor_follow_link" },

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
 # 'f' translates to arrow key right
 { key = "alt+f", command = "note_editor_experimental_cursor_word_forward" },
 # 'b' translates to arrow key left
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
 # 'f' translates to arrow key right
 { key = "alt+f", command = "input_modal_word_forward" },
 # 'b' translates to arrow key left
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

[debug_log_modal]
key_bindings = [
 { key = "esc", command = "debug_log_close" },
 { key = "k", command = "debug_log_scroll_up_one" },
 { key = "j", command = "debug_log_scroll_down_one" },
 { key = "up", command = "debug_log_scroll_up_one" },
 { key = "down", command = "debug_log_scroll_down_one" },
 { key = "ctrl+u", command = "debug_log_scroll_up_half_page" },
 { key = "ctrl+d", command = "debug_log_scroll_down_half_page" },
 { key = "c", command = "debug_log_clear" },
 { key = "l", command = "debug_log_cycle_level" },
]

[theme_selector_modal]
key_bindings = [
 { key = "k", command = "theme_selector_modal_up" },
 { key = "j", command = "theme_selector_modal_down" },
 { key = "up", command = "theme_selector_modal_up" },
 { key = "down", command = "theme_selector_modal_down" },
 { key = "enter", command = "theme_selector_modal_open" },
 { key = "esc", command = "theme_selector_modal_close" },
]
```
