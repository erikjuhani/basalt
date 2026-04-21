Hide
Sleep 1s
Enter
Sleep 1s
Type "cargo run --release -q"
Enter
Sleep 5s
Show
Sleep 1s

# Navigate to Basalt.md (5th item: 4 folders then first file)
Type "j"
Sleep 1s
Type "j"
Sleep 1s
Type "j"
Sleep 1s
Type "j"
Sleep 1s

# Rename Basalt to Basalt Companion
Type "r"
Sleep 1s
Space
Sleep 1s
Type@300ms "Companion"
Sleep 2s
Enter
Sleep 3s

# Open the renamed note
Enter
Sleep 3s

# Move to editor pane
Tab
Sleep 2.5s

# Close explorer pane
Type "t"
Sleep 3.5s

# Open outline pane
Ctrl+O
Sleep 4s

# Switch to outline
Tab
Sleep 1s

# Navigate outline headings
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 1.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 1s
Type "j"
Sleep 1.5s
Type "g"
Sleep 2.5s
Shift+Tab
Sleep 1
Type "gg"
Sleep 2

# Enter edit mode
Type "i"
Sleep 2.0s
Enter
Sleep 1.5s
Enter
Sleep 0.5s
Up
Sleep 1s
Up
Sleep 2s
Type@300ms "### What is it?"
Sleep 2s
Escape
Sleep 3s

# Enter edit mode again
Type "i"
Sleep 2s
Down
Sleep 1.5s
Left
Left
Left
Left
Left
Left
Left
Left
Left
Left
Left
Left
Left
Left
Left
Enter
Sleep 1.5s
Enter
Sleep 1.5s
Up
Sleep 1.0s
Up
Sleep 1.0s
Type@200ms "Basalt is a mafic, extrusive igneous rock formed from the rapid cooling of viscosity lava rich in magnesium and iron."
Sleep 3s
Escape
Sleep 3s

# Save
Ctrl+X
Sleep 5s

# Switch to read view
Shift+R
Sleep 5s

# Open outline
Ctrl+O
Sleep 6s

# Open help modal
Type "?"
Sleep 4s
Ctrl+D
Sleep 2s
Ctrl+D
Sleep 3s
Ctrl+D
Sleep 7s

# Quit
Type "q"

# Cleanup
Hide
Type "git checkout docs/"
Enter
Type "rm docs/Basalt\ Companion.md"
Enter
