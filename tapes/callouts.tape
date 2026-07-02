Hide
# Create a note that showcases callout blocks
Type "printf '> [!note]\n> Callouts render with an icon and a coloured header.\n\n> [!tip]- Did you know?\n> Any callout can have a custom title.\n\n> [!success]\n> Everything went well.\n\n> [!warning]\n> Be careful with this step.\n\n> [!danger]\n> This could go wrong.\n\n> [!quote]\n> Simplicity is the ultimate sophistication.\n' > docs/Callouts.md"
Enter
Type "cargo run --release -q"
Enter
Sleep 3s
Show
Sleep 1s

# Open the Callouts note (5 down: 4 folders, Basalt, then Callouts)
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

# Focus the editor, hide the explorer, and switch to read view so every
# callout renders fully (the cursor's line stays raw in edit view).
Tab
Sleep 1s
Type "t"
Sleep 0.5s
Shift+R
Sleep 4s

# Quit (without saving)
Type "q"

# Cleanup
Hide
Type "rm docs/Callouts.md"
Enter
