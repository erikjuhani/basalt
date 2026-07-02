Hide
# Create a note that explains visual selection
Type "printf '# Visual selection\n\nYank text straight to your clipboard.\n\n- Press v for a charwise selection\n- Press V to grab whole lines\n- Press y to yank and esc to cancel\n' > 'docs/Visual selection.md'"
Enter
Type "cargo run --release -q"
Enter
Sleep 2s
Show
Sleep 1.5s

# Jump to the bottom of the explorer to select the new note
Shift+G
Sleep 1.2s
Enter
Sleep 1.2s

# Move into the editor pane, enter edit mode then Esc to Normal
Tab
Sleep 1s
Type "i"
Sleep 1.2s
Escape
Sleep 1.2s

# Charwise: v starts a selection, word motions extend it, y yanks.
# A short flash and a toast confirm the copy.
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.8s
Type "v"
Sleep 0.8s
Type@250ms "www"
Sleep 1s
Type "y"
Sleep 2s

# Linewise: V selects whole lines, j extends the range down, y yanks.
Type "V"
Sleep 0.9s
Type@450ms "jj"
Sleep 1.3s
Type "y"
Sleep 2s

# Quit (without saving)
Type "q"

# Cleanup
Hide
Type "rm 'docs/Visual selection.md'"
Enter
