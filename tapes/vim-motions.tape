Hide
# A tutorial note: each block quote names the keys, the plain line below is the
# practice text the cursor then performs them on.
Type `printf '# Vim motions\n\n> gg and G jump to both ends of the note.\n\nWatch the cursor leap around.\n\n> w b e move by word; 0 $ jump to ends.\n\nThe quick brown fox jumps over lazy dogs.\n\n> f finds a char; ; and , repeat it.\n\nLook for the letter o on this row of text.\n\n> dw deletes a word, cw changes it, u undo.\n\ndelete or change the first word here now.\n\n> r replaces a char, dd deletes a line.\n\nreplace the first letter of this line.\n\n> yy yanks a line, p pastes it below.\n\ncopy this line and paste a duplicate.\n\n> ci" edits quotes, da( edits parens.\n\nedit "a quoted phrase" and (some parens).\n\n> v selects, V lines, y yanks, d deletes.\n\nselect these first three words to yank.\n' > 'docs/Vim motions.md'`
Enter
Type "cargo run --release -q"
Enter
Sleep 2s
Show
Sleep 1.5s

# Open the note; with vim_mode it opens straight into NORMAL mode.
Shift+G
Sleep 1s
Enter
Sleep 1s
Tab
Sleep 1.5s

# gg / G / { / } — land on this section's line, then leap around.
Type@200ms "5G"
Sleep 1.2s
Type "G"
Sleep 1.2s
Type "gg"
Sleep 1.2s
Type@400ms "}}"
Sleep 1s
Type "{"
Sleep 1.2s

# w / b / e / 0 / $ — jump to the fox line, then move across it.
Type@200ms "9G"
Sleep 1s
Type@450ms "www"
Sleep 0.8s
Type "b"
Sleep 0.7s
Type "e"
Sleep 0.8s
Type "$"
Sleep 0.8s
Type "0"
Sleep 1.2s

# f / ; / , — find a character, repeat and reverse.
Type@200ms "13G"
Sleep 1s
Type@250ms "fo"
Sleep 0.9s
Type ";"
Sleep 0.7s
Type ";"
Sleep 0.7s
Type ","
Sleep 1.2s

# dw / cw / u — delete then change the first word, undoing each.
Type@200ms "17G"
Sleep 1s
Type@250ms "dw"
Sleep 1s
Type "u"
Sleep 1s
Type@250ms "cw"
Sleep 0.6s
Type@120ms "swap"
Sleep 0.8s
Escape
Sleep 0.8s
Type "u"
Sleep 1.2s

# r / dd / u — replace a character, delete a line.
Type@200ms "21G"
Sleep 1s
Type@250ms "rX"
Sleep 1s
Type "u"
Sleep 1s
Type@250ms "dd"
Sleep 1s
Type "u"
Sleep 1.2s

# yy / p / u — yank a line and paste a duplicate.
Type@200ms "25G"
Sleep 1s
Type@250ms "yy"
Sleep 0.8s
Type "p"
Sleep 1.1s
Type "u"
Sleep 1.2s

# ci" / da( — edit inside a text object, then undo.
Type@200ms "29G"
Sleep 1s
Type `ci"`
Sleep 0.6s
Type@120ms "quoted words"
Sleep 0.8s
Escape
Sleep 0.8s
Type "u"
Sleep 0.9s
Type "f("
Sleep 0.7s
Type@250ms "da("
Sleep 1s
Type "u"
Sleep 1.2s

# v / y — visual select and yank to the clipboard.
Type@200ms "33G"
Sleep 1s
Type "v"
Sleep 0.6s
Type@350ms "ww"
Sleep 0.8s
Type "y"
Sleep 2s

# Quit without saving.
Hide
Type "q"
Sleep 0.5s
# Cleanup
Type "rm 'docs/Vim motions.md'"
Enter
