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
- [x] Add Side panel for note selection in vault
- [x] Add bottom information bar that shows the current mode Select, Normal, Insert and statistics for words and characters
- [x] Add help modal / popup with `?`
- [x] Add vault selection screen with basalt logo (Splash screen)
- [x] Add vault selector modal
- [-] GitHub Workflows !
    - [x] Run tests and build
    - [x] Run create release artifacts (cross-platform binaries)
    - [ ] Do not run test when pushing a tag
    - [ ] Run `vhs` when basalt directory changes and commit it to the current PR
- [ ] Add `mdbook` and `gh` pages
- [ ] Persistent scroll state in help modal
- [ ] Fuzzy search in panes (note, side panel, modals)
- [ ] Markdown rendering
    - [ ] Add support to all markdown nodes
    - [ ] Improve and fix code block rendering, so it appears as a 'block'
    - [x] Add text formatting to different styles like `Fraktur` and `DoubleStruck` for heading purposes
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
    - [ ] Change to raw text where cursor is. Only changes the current markdown node. Text is inserted node by node.
    - [ ] Edit and save notes
    - [ ] Support some vim keybindings to get started (vim mode should be configurable option)
    - [ ] Easy text yanking
- [ ] Command bar
    - [ ] Add ability to invoke command bar with `:`
    - [ ] Add commands for saving `:w` and quitting `:q`
    - [ ] Switch between scrollbar and paging using a command `:set scroll` or `:set paging`. Paging will only fit the content it can within the height of the `rect` and generate pages accordingly.
- [ ] Configuration file (`.basalt.toml`)
    - [ ] Add rudimentary configuration file and move key bindings to the file
- [x] Wrap lines with prefix (calculate width and add length of prefix)
- [ ] Easy backups with Git (Config, (git2-rs)[https://github.com/rust-lang/git2-rs])
- [ ] Integration tests using https://core.tcl-lang.org/expect/index
- [ ] When creating a link show autocomplete tooltip list of potential files to link to
- [ ] Add features to basalt-core and basalt-widgets. Default feature set and individual features.

## Contribution Policy (I want to help)

Basalt is open for code contributions, but mainly for bug fixes. Feature work can bring long-term maintenance overhead, and I'd like to keep that to a minimum. One big reason for limiting feature work is that I want to build the features myself as this is my _FUN_ project alongside work, and I want to keep it that way.

However, I do realize that open source projects usually flourish from multiple contributors, thus, I won't say no, if you would like to contribute on feature work, but please, open an issue first, so we can chat about it. That way we can avoid unnecessary effort or bike shedding over architectural or stylistic choices. I have my own opinions and ideas on how certain things should be written in this project.

I want this project to feel low-barrier so don't be discouraged to open an issue, be it about existing features, ideas, or anything really!

Furthermore, I will make a proper contribution's guideline a bit later, with more details on certain operational things of this project.

If you find mistakes in the documentation or simple fixes in code please go ahead and open a pull request with the changes!

### Contributing

1. Make a fork from `basalt`
2. Create a branch
3. Open a pull-request against basalt's main branch with your changes
4. I'll review your pull-request as soon as possible and either leave comments or merge it

#### I Found a Bug

Know how to fix it? - Open a PR with the fix.

Not sure or you don't want to do it yourself? - Open an issue with steps to reproduce.

#### I Found a Typo

Open a PR directly with the correction.

#### I Want To Contribute on a Feature

Open an issue first, so we can chat about the feature work!

### Git Pre-push Hook

There's a useful pre-push git-hook under `scripts`, which you can enable by running the following command:

```
cp scripts/pre-push .git/hooks/
```

The script runs the same test commands as in the `test.yml` workflow.
