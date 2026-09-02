
# Alias the release binary as `basalt` so the visible command reads cleanly.
Hide
Type 'alias basalt "./target/release/basalt --theme $BASALT_THEME"'
Enter
Type "clear"
Enter
Sleep 0.5s
Show

Sleep 1.5s

# Single-file mode takes the file path as an argument, so typing the command
# is where you choose the file.
Type "basalt README.md"
Sleep 3s
Enter

# The file opens straight into the note editor with the outline on the right.
# Single-file mode has no explorer pane.
Sleep 3.5s

# Scroll through the rendered markdown
Ctrl+D
Sleep 1.5s
Ctrl+D
Sleep 1.5s
Ctrl+U
Sleep 1.5s

# Open the outline pane
Ctrl+O
Sleep 1.5s

# Focus the outline
Tab
Sleep 1.5s

# Navigate headings
Type "j"
Sleep 0.6s
Type "j"
Sleep 0.6s
Type "j"
Sleep 1s

# Jump to the selected heading
Type "g"
Sleep 2.5s

# Cycle focus back to the editor (no explorer to pass through)
Tab
Sleep 2s

# Quit
Type "q"
