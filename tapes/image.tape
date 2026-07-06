Hide
# Seed the vault with two images (in a subfolder, so they stay out of the
# explorer) and a note that embeds the first
Type "mkdir -p docs/attachments && cp assets/basalt.png docs/attachments/basalt.png && sips --rotate 180 assets/basalt.png --out docs/attachments/basalt2.png"
Enter
Type "printf '![[basalt.png]]\n\n# Inline images\n\nBasalt renders the embedded image above inline, scaled to the note width. It scrolls with the note, rendering partially as it passes through the viewport.\n' > docs/Images.md"
Enter
Type "cargo run --release -q"
Enter
Sleep 3s
Show
Sleep 1s

# Open the Images note (item 10: 5 folders, then Basalt, Editing, Files, Home, Images)
Type@250ms "jjjjjjjjj"
Sleep 1s
Enter
Sleep 2s

# Focus the note editor and switch to read view so the image renders fully
Tab
Sleep 1s
Shift+R
Sleep 2.5s

# Scroll down half a page at a time: the image renders partially as it
# passes up through the viewport, then the note continues below it
Ctrl+D
Sleep 1.8s
Ctrl+D
Sleep 1.8s
Ctrl+D
Sleep 2s

# Scroll back up to see it re-enter from the top
Ctrl+U
Sleep 1.5s
Ctrl+U
Sleep 1.5s

# Edit the embed (first line) to point at a different image
Ctrl+Shift+Up
Sleep 1s
Type "i"
Sleep 1.2s
Right 9
Sleep 0.6s
Type@300ms "2"
Sleep 1.5s
Escape
Sleep 0.8s
Shift+R
Sleep 4s

# Quit without saving
Type "q"

# Cleanup
Hide
Type "rm -r docs/Images.md docs/attachments"
Enter
