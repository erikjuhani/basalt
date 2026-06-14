Hide
# Create a small note that showcases line-by-line rendering
Type "printf '# Line by line editing\n\n> Basalt reveals the raw markdown only on the line you edit.\n> Every other line stays rendered.\n\n- Fruits\n  - apples\n  - bananas\n- Vegetables\n  - carrots\n' > docs/Demo.md"
Enter
Type "cargo run --release -q"
Enter
Sleep 3s
Show
Sleep 1s

# Open the demo note (5 down: 4 folders, Basalt, then Demo)
Type "j"
Sleep 0.4s
Type "j"
Sleep 0.4s
Type "j"
Sleep 0.4s
Type "j"
Sleep 0.4s
Type "j"
Sleep 1s
Enter
Sleep 2s

# Move into the editor pane
Tab
Sleep 1.5s

# Enter edit mode: only the cursor's line shows its raw markdown,
# every other line stays rendered.
Type "i"
Sleep 2s

# Walk down through the block quote and the nested list, line by line.
# The current line reveals its `>` / `-` markers, the rest stay rendered.
Down
Sleep 1.2s
Down
Sleep 1.2s
Down
Sleep 1.2s
Down
Sleep 1.2s
Down
Sleep 1.2s
Down
Sleep 1.2s
Down
Sleep 1.2s
Down
Sleep 1.5s

# Edit a list item in place
Type@200ms "baby "
Sleep 2s
Escape
Sleep 2s

# Switch to read view to see it fully rendered
Shift+R
Sleep 3s

# Quit (without saving)
Type "q"

# Cleanup
Hide
Type "rm docs/Demo.md"
Enter
