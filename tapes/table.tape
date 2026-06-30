Hide
# Create a small table note for the demo
Type "printf '| Task | Done |\n| --- | --- |\n| Ship | yes |\n| Docs | no |\n' > docs/Tables.md"
Enter
Type "cargo run --release -q"
Enter
Sleep 2s
Show
Sleep 1.5s

# Jump to the bottom of the explorer (Shift+G) to select the Tables note
Shift+G
Sleep 1.5s
Enter
Sleep 1.2s

# Editor pane: the table is drawn as a bordered box with sized columns
Tab
Sleep 1.5s

# Edit the delimiter to make the first column right aligned
Type "i"
Sleep 0.6s
Down
Sleep 0.7s
Right 5
Type@150ms ":"
Sleep 1.2s

# Edit a cell so its text grows past the column and wraps onto more rows
Down
Sleep 0.7s
Right 6
Type@45ms " the tables, write docs and tag the release"
Sleep 1.5s
Escape
Sleep 0.6s

# Read view: the first column is right aligned and the long cell wraps
Shift+R
Sleep 2s

# Quit (without saving)
Type "q"

# Cleanup
Hide
Type "rm docs/Tables.md"
Enter
