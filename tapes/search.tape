
Hide
Type "cargo run --release -q -- --theme $BASALT_THEME"
Enter
Sleep 3s
Show
Sleep 1s

# Open full-text search across the vault
Space
Type "s"
Sleep 1.5s

# Type a query. Results stream in and re-rank on each keystroke.
Type "t"
Sleep 0.4s
Type "h"
Sleep 0.4s
Type "e"
Sleep 0.4s
Type "m"
Sleep 0.4s
Type "e"
Sleep 2s

# Move the selection through the matches
Ctrl+N
Sleep 0.8s
Ctrl+N
Sleep 0.8s
Ctrl+N
Sleep 1.5s

# Open the selected note at the matched line
Enter
Sleep 3s

# Quit
Type "q"
