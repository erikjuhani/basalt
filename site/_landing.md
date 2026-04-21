+++
title = "basalt"
description = "TUI Application to manage Obsidian vaults and notes directly from the terminal."
template = "index.html"
sort_by = "weight"

[extra]
tagline_line1 = "TUI Application to manage Obsidian"
tagline_line2 = "vaults and notes directly from the terminal."
demo_gif = "demo"

[[extra.install_commands]]
label = "cargo"
cmd = "cargo install basalt-tui"

[[extra.install_commands]]
label = "aqua"
cmd = "aqua g -i erikjuhani/basalt"

[[extra.cards]]
title = "Getting started"
href = "@/getting-started/_index.md"
blurb = "Install and open your first vault"

[[extra.cards]]
title = "User interface"
href = "@/user-interface/_index.md"
blurb = "Panes, modals, navigation"

[[extra.cards]]
title = "Configuration"
href = "@/configuration/_index.md"
blurb = "Keymaps, commands, integrations"

[[extra.cards]]
title = "Editing and Formatting"
href = "@/editing-and-formatting.md"
blurb = "Markdown rendering"
+++

basalt is a cross-platform TUI for managing Obsidian vaults and notes. It runs on Windows, macOS, and Linux — a minimalist terminal companion with a WYSIWYG-style reading experience.

basalt is not a replacement for Obsidian. It's a terminal-native way to read, browse, and edit your notes without leaving the shell.
