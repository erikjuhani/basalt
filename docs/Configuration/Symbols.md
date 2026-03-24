Basalt uses symbols throughout the interface for tree indicators, list markers, heading decorations and other visual elements. You can customize these through the `[symbols]` table in your [[Configuration|configuration file]].

## Presets

Symbols start from a **preset**, a complete set of defaults. Pick one and optionally override individual fields.

| Preset      | Description                                                  |
| ----------- | ------------------------------------------------------------ |
| `unicode`   | Rich Unicode glyphs (default). Works in most modern terminals. |
| `ascii`     | Plain ASCII characters. Use when your terminal has limited font support. |
| `nerd-font` | Uses [Nerd Font](https://www.nerdfonts.com/) icons for file and folder indicators. Requires a Nerd Font installed. |

Set the preset in your config:

```toml
[symbols]
preset = "unicode"
```

## Overriding individual symbols

Any field can be overridden on top of the chosen preset. Only specify the symbols you want to change; all others keep their preset values.

```toml
[symbols]
preset = "nerd-font"
task_checked = "[x]"
task_unchecked = "[ ]"
```

## Symbol reference

### General

| Field              | Description                            | Unicode   | ASCII     | Nerd Font |
| ------------------ | -------------------------------------- | --------- | --------- | --------- |
| `wrap_marker`      | Shown at the start of wrapped lines    | `⤷ `      | *(empty)* | `⤷ `      |
| `horizontal_rule`  | Character used for horizontal rules    | `═`       | `=`       | `═`       |
| `pane_open`        | Indicator for an open pane             | `▶`       | `>`       | `▶`       |
| `pane_close`       | Indicator for a closed pane            | `◀`       | `<`       | `◀`       |
| `pane_full`        | Indicator for a full-width pane        | `⟹ `      | `=>`      | `⟹ `      |

### Explorer

| Field                        | Description                             | Unicode | ASCII | Nerd Font          |
| ---------------------------- | --------------------------------------- | ------- | ----- | ------------------ |
| `tree_indent`                | Vertical line for tree indentation      | `│`     | `\|`  | `│`                |
| `tree_expanded`              | Expanded folder indicator               | `▾`     | `v`   | (folder-open icon) |
| `tree_collapsed`             | Collapsed folder indicator              | `▸`     | `>`   | (folder icon)      |
| `selected`                   | Selected file indicator                 | `◆`     | `*`   | (file icon)        |
| `unselected`                 | Unselected file indicator               | `◦`     | `.`   | (dot icon)         |
| `sort_asc`                   | Ascending sort indicator                | `↑≡`    | `^=`  | (sort-asc icon)    |
| `sort_desc`                  | Descending sort indicator               | `↓≡`    | `v=`  | (sort-desc icon)   |
| `folder_expanded_collapsed`  | Expanded folder in collapsed pane view  | `▪`     | `+`   | (folder-open icon) |
| `folder_collapsed_collapsed` | Collapsed folder in collapsed pane view | `▫`     | `-`   | (folder icon)      |
| `heading_collapsed_dot`      | Heading dot in collapsed explorer       | `·`     | `.`   | `·`                |

### Note editor

| Field               | Description                       | Unicode  | ASCII    | Nerd Font              |
| -------------------- | -------------------------------- | -------- | -------- | ---------------------- |
| `h1_underline`       | Character used for H1 underline  | `═`      | `=`      | `═`                    |
| `h2_underline`       | Character used for H2 underline  | `─`      | `-`      | `─`                    |
| `h3_marker`          | Marker symbol for H3 headings    | `◉`      | `###`    | `◉`                    |
| `h4_marker`          | Marker symbol for H4 headings    | `◎`      | `####`   | `◎`                    |
| `h5_marker`          | Marker symbol for H5 headings    | `◈`      | `#####`  | `◈`                    |
| `h6_marker`          | Marker symbol for H6 headings    | `✦`      | `######` | `✦`                    |
| `task_unchecked`     | Unchecked task marker             | `□`      | `[ ]`    | (checkbox-blank icon)  |
| `task_checked`       | Checked task marker               | `■`      | `[x]`    | (checkbox-marked icon) |
| `blockquote_border`  | Border character for block quotes | `┃`      | `\|`     | `┃`                    |

### List markers

List markers cycle based on nesting depth. Configure them as an array:

```toml
[symbols]
list_markers = ["*", "-", "+"]
```

| Preset      | Default markers            |
| ----------- | -------------------------- |
| `unicode`   | `["●", "○", "◆", "◇"]`    |
| `ascii`     | `["-", "*", "+"]`          |
| `nerd-font` | `["●", "○", "◆", "◇"]`    |

At depth 0 the first marker is used, at depth 1 the second, and so on. When the depth exceeds the number of markers the list wraps around.

### Outline

| Field                        | Description                            | Unicode | ASCII | Nerd Font |
| ---------------------------- | -------------------------------------- | ------- | ----- | --------- |
| `outline_indent`             | Indentation character for nested items | `│`     | `\|`  | `│`       |
| `outline_expanded`           | Expanded heading indicator             | `▾`     | `v`   | `▾`       |
| `outline_collapsed`          | Collapsed heading indicator            | `▸`     | `>`   | `▸`       |
| `outline_heading_dot`        | Heading dot in collapsed outline       | `·`     | `.`   | `·`       |
| `outline_heading_expanded`   | Expanded heading in collapsed outline  | `✺`     | `v`   | `✺`       |
| `outline_heading_collapsed`  | Collapsed heading in collapsed outline | `◦`     | `>`   | `◦`       |

## Font styles

Some text elements can have a decorative font style applied. The available styles are `black-board-bold`, `fraktur-bold` and `script`.

| Field              | Description                   | Unicode / Nerd Font  | ASCII    |
| ------------------ | ----------------------------- | -------------------- | -------- |
| `title_font_style` | Font style for the note title | `black-board-bold`   | *(none)* |
| `h5_font_style`    | Font style for H5 headings    | `script`             | *(none)* |
| `h6_font_style`    | Font style for H6 headings    | `script`             | *(none)* |

Set to a style name to enable, or omit to use no special styling:

```toml
[symbols]
title_font_style = "fraktur-bold"
h5_font_style = "script"
```

## Full example

```toml
[symbols]
preset = "unicode"
task_checked = "[x]"
task_unchecked = "[ ]"
list_markers = ["->", "=>", "~>"]
h5_font_style = "fraktur-bold"
```

This uses the `unicode` preset but replaces the task markers with ASCII-style checkboxes, uses custom list markers and changes H5 headings to blackboard bold style.
