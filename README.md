<img align="left" width="125px" src="assets/basalt.png">
<h3>Basalt&nbsp;&nbsp;</h3>
<p>TUI Application to manage Obsidian notes&nbsp;&nbsp;&nbsp;&nbsp;</p>

<hr/>

TUI Application to manage Obsidian vaults and notes directly from the terminal ✨.

<img src="assets/basalt_demo.gif">

## Installation

For now unfortunately, you have to compile this binary yourself, if you want to run basalt. The next thing I'll be doing is adding a GitHub workflow to produce cross-platform binaries.

Install basalt using cargo:

```sh
cargo install basalt-tui
```

## Background

This is something that has been brewing in my head for quite some time. There has been different incarnations over the years, however, nothing as substantial as this.

I have been using Neovim and the official Obsidian app. However, I wanted to have something dedicated that offers the same writing experience as Neovim, but has more WYSIWYG experience as in the official Obsidian app. I'm fully aware of (obsidian.nvim)[https://github.com/epwalsh/obsidian.nvim], which many people use and find more than sufficient. However, I want to see images, beautified text, note graphs, etc. I want it to be a bit more.

The problem for me personally is that when I leave the terminal, my flow breaks, especially if I'm writing. Using an entirely different app disrupts that flow, and it _annoys_ me. So here I am, building a TUI for Obsidian.

The goal of basalt is not to replace the Obsidian app. Basalt is to fill and cater for a need to have a terminal view to the selection of notes and vaults, providing quick access from anywhere in the terminal with a simple command.

## Keybindings

For now these are not configurable, but this will change when the configuration file is supported.

<kbd>q</kbd> Quit the application

<kbd>?</kbd> Show help

<kbd>t</kbd> Toggle side panel visibility and select mode

<kbd>k</kbd> Move selection up

<kbd>j</kbd> Move selection down

<kbd>↑ / ↓</kbd> Scroll selected up / down

<kbd>↩ Enter</kbd> Select the highlighted note

<kbd>Space</kbd> Toggle vault selector modal

<kbd>Ctrl-u</kbd> Scroll up half a page

<kbd>Ctrl-d</kbd> Scroll down half a page

## Task List

- [x] Add rudimentary support for markdown rendering
- [x] Add Sidepanel for note selection in vault
- [x] Add bottom information bar that shows the current mode Select, Normal, Insert and statistics for words and characters
- [x] Add help modal / popup with `?`
- [x] Add vault selection screen with basalt logo (Splash screen)
- [x] Add vault selector modal
- [ ] GitHub Workflows !
    - [x] Run tests and build
    - [x] Run create release artifacts (cross-platform binaries)
    - [ ] Do not run test when pushing a tag
    - [ ] Run vhs when basalt dir changes and commit it to the current PR
- [ ] Add mdbook and gh pages
- [ ] Async file loading (tokio)
- [ ] Persistent scroll state in help modal
- [ ] Fuzzy search in panes (note, sidepanel, modals)
- [ ] Markdown rendering
    - [ ] Add support to all markdown nodes
    - [ ] Add text formatting to different styles like Fraktur and DoubleStruck for heading purposes
    - [ ] Improve and fix codeblock rendering so it appears as a 'block'
    - [ ] Support complete Obsidian Flavor
    - [ ] Add image rendering support
- [ ] Note tree
    - [ ] Create new note under vault
    - [ ] Collapsible folders
    - [ ] Notes within Folders in vault
    - [ ] Move note
    - [ ] Rename note
    - [ ] Delete note under vault (with confirmation modal)
- [ ] Editor mode
    - [ ] Editor mode should change to raw text where cursor is. Only changes the current markdown node. Text is inserted node by node.
    - [ ] Edit and save notes
    - [ ] Support some vim keybindings to get started (vim mode should be configurable option)
    - [ ] Easy text yanking
- [ ] Command bar
    - [ ] Add ability to invoke command bar with `:`
    - [ ] Add commands for saving `:w` and quitting `:q`
    - [ ] Switch between scrollbar and paging using a command `:set scroll` or `:set paging`. Paging will only fit the content it can within the height of the rect and generate pages accordingly.
- [ ] Configuration file (.basalt.toml)
    - [ ] Add rudimentary config file and move keybinds to the file
- [ ] Wrap lines with prefix (calculate width and add length of prefix)
- [ ] Easy backups with Git (Config, (git2-rs)[https://github.com/rust-lang/git2-rs])
- [ ] Integration tests using https://core.tcl-lang.org/expect/index
- [ ] When creating a link show autocomplete tooltip list of potential files to link to
- [ ] Add features to basalt-core and basalt-widgets. Default feature set and individual features.
- [ ] Clickable checkboxes

## I want to help

I haven't yet had the chance to add a contributors' guide. If you would like to help, please feel free to create a pull request directly. At this stage, opening a separate issue is not required unless you would like to start a discussion first.

Please note that this process may change in the future. The expected contribution flow will likely become: Create issue → Create pull request.

There's a useful pre-push git-hook under `scripts`, which you can enable by running the following command:

```
cp scripts/pre-push .git/hooks/
```

The script runs the same test commands as in the `test.yml` workflow.
