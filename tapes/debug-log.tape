
Hide
Type "cargo run --release -q"
Enter
Sleep 3s
Show
Sleep 1.5s

# Navigate to Known Limitations.md (9 down)
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 1.5s

# Rename it, which refreshes the vault and runs the wiki-link update
Type "r"
Sleep 1.5s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 0.1s
Backspace
Sleep 1s
Type@250ms "Caveats"
Sleep 2s
Enter
Sleep 2.5s

# Open the debug log overlay
Type "g<"
Sleep 3s

# Scroll up slowly to read earlier entries
Type "k"
Sleep 1.5s
Type "k"
Sleep 1.5s
Type "k"
Sleep 2s

# Filter to warnings and above
Type "L"
Sleep 1.5s
Type "L"
Sleep 1.5s
Type "L"
Sleep 3s

# Close the overlay
Escape
Sleep 2s

# Quit
Type "q"

# Cleanup
Hide
Type "git checkout 'docs/' && rm -f 'docs/Caveats.md'"
Enter
