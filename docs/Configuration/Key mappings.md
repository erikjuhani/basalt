Key mappings can be modified or extended by defining them in the [[Configuration|configuration file]].

Each key mapping is associated with a specific pane and becomes active when that pane has focus. The `global` section applies to all panes and is evaluated first.

```toml
[global]
key_bindings = [
  { key = "q", command = "quit" },
  { key = "?", command = "help_modal_toggle" },
]

[explorer]
key_bindings = [
  { key = "k", command = "explorer_up" },
  { key = "j", command = "explorer_down" },
]
```

## Key sequence syntax

A key can be a single character, a named key, a modified key, or a **sequence** of keystrokes. Sequences are written as a multi-character string and only fire when all keys are pressed in order with nothing in between. This makes it possible to define vim- or Helix-style bindings like `gg`.

```toml
[note_editor]
key_bindings = [
  { key = "gg", command = "note_editor_scroll_to_top" },
  { key = "G",  command = "note_editor_scroll_to_bottom" },
]
```

An uppercase letter like `G` is shorthand for `shift+g` — shift is implied automatically.

## Available commands

### Global commands

| Command                       | Description                          |
| ----------------------------- | ------------------------------------ |
| `quit`                        | Exit the application                 |
| `vault_selector_modal_toggle` | Toggle vault selector modal          |
| `help_modal_toggle`           | Toggle help modal                    |

### Splash commands

| Command        | Description              |
| -------------- | ------------------------ |
| `splash_up`    | Move selector up         |
| `splash_down`  | Move selector down       |
| `splash_open`  | Open the selected vault  |

### Explorer commands

| Command                          | Description                                    |
| -------------------------------- | ---------------------------------------------- |
| `explorer_up`                    | Move selector up                               |
| `explorer_down`                  | Move selector down                             |
| `explorer_open`                  | Open selected note in note editor              |
| `explorer_sort`                  | Toggle sort between A-z and Z-a                |
| `explorer_toggle`                | Toggle explorer pane                           |
| `explorer_toggle_outline`        | Toggle outline pane                            |
| `explorer_toggle_input_rename`   | Open rename dialog for selected item           |
| `explorer_hide_pane`             | Hide pane (stepped)                            |
| `explorer_expand_pane`           | Expand pane (stepped)                          |
| `explorer_switch_pane_next`      | Switch focus to next pane                      |
| `explorer_switch_pane_previous`  | Switch focus to previous pane                  |
| `explorer_scroll_up_one`         | Scroll selector up by one                      |
| `explorer_scroll_down_one`       | Scroll selector down by one                    |
| `explorer_scroll_up_half_page`   | Scroll selector up half a page                 |
| `explorer_scroll_down_half_page` | Scroll selector down half a page               |
| `explorer_scroll_to_top`         | Jump to the first item                         |
| `explorer_scroll_to_bottom`      | Jump to the last item                          |

### Outline commands

| Command                         | Description                                     |
| ------------------------------- | ----------------------------------------------- |
| `outline_up`                    | Move selector up                                |
| `outline_down`                  | Move selector down                              |
| `outline_toggle`                | Toggle outline pane                             |
| `outline_toggle_explorer`       | Toggle explorer pane                            |
| `outline_switch_pane_next`      | Switch focus to next pane                       |
| `outline_switch_pane_previous`  | Switch focus to previous pane                   |
| `outline_expand`                | Expand or collapse heading                      |
| `outline_select`                | Jump to heading in editor                       |

### Note editor commands

| Command                                | Description                         |
| -------------------------------------- | ----------------------------------- |
| `note_editor_cursor_up`                | Move cursor up                      |
| `note_editor_cursor_down`              | Move cursor down                    |
| `note_editor_scroll_up_one`            | Scroll up by one                    |
| `note_editor_scroll_down_one`          | Scroll down by one                  |
| `note_editor_scroll_up_half_page`      | Scroll up half page                 |
| `note_editor_scroll_down_half_page`    | Scroll down half page               |
| `note_editor_scroll_to_top`            | Jump to the top of the note         |
| `note_editor_scroll_to_bottom`         | Jump to the bottom of the note      |
| `note_editor_toggle_explorer`          | Toggle explorer pane                |
| `note_editor_toggle_outline`           | Toggle outline pane                 |
| `note_editor_switch_pane_next`         | Switch focus to next pane           |
| `note_editor_switch_pane_previous`     | Switch focus to previous pane       |

### Experimental editor commands

| Command                                          | Description                    |
| ------------------------------------------------ | ------------------------------ |
| `note_editor_experimental_set_edit_view`          | Switch to edit view            |
| `note_editor_experimental_toggle_view`            | Toggle between edit and read   |
| `note_editor_experimental_set_read_view`          | Switch to read view            |
| `note_editor_experimental_save`                   | Save note changes              |
| `note_editor_experimental_exit`                   | Cancel editing, switch to read |
| `note_editor_experimental_cursor_left`            | Move cursor left               |
| `note_editor_experimental_cursor_right`           | Move cursor right              |
| `note_editor_experimental_cursor_word_forward`    | Move cursor forward by word    |
| `note_editor_experimental_cursor_word_backward`   | Move cursor backward by word   |

### Input modal commands

| Command                    | Description                     |
| -------------------------- | ------------------------------- |
| `input_modal_edit_mode`    | Enter edit mode for typing      |
| `input_modal_accept`       | Accept changes and close modal  |
| `input_modal_cancel`       | Cancel and close modal          |
| `input_modal_left`         | Move cursor left                |
| `input_modal_right`        | Move cursor right               |
| `input_modal_word_forward` | Move cursor forward by word     |
| `input_modal_word_backward`| Move cursor backward by word    |

### Help modal commands

| Command                           | Description              |
| --------------------------------- | ------------------------ |
| `help_modal_toggle`               | Toggle help modal        |
| `help_modal_close`                | Close help modal         |
| `help_modal_scroll_up_one`        | Scroll up by one         |
| `help_modal_scroll_down_one`      | Scroll down by one       |
| `help_modal_scroll_up_half_page`  | Scroll up half page      |
| `help_modal_scroll_down_half_page`| Scroll down half page    |

### Vault selector modal commands

| Command                        | Description                     |
| ------------------------------ | ------------------------------- |
| `vault_selector_modal_up`      | Move selector up                |
| `vault_selector_modal_down`    | Move selector down              |
| `vault_selector_modal_close`   | Close vault selector modal      |
| `vault_selector_modal_open`    | Open selected vault             |
| `vault_selector_modal_toggle`  | Toggle vault selector modal     |
