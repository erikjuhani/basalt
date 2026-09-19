Hide
# Create a note with a fenced Rust code block
Type "printf '# Syntax highlighting\n\nName a language on the fence and the code gets colours.\n\n\140\140\140rust\nfn greet(name: &str) -> String {\n    // Build the greeting\n    format!(\042Hello, {name}!\042)\n}\n\nfn main() {\n    let greeting = greet(\042Basalt\042);\n    println!(\042{greeting}\042);\n}\n\140\140\140\n' > docs/Code.md"
Enter
Type "cargo run --release -q -- --theme $BASALT_THEME"
Enter
Sleep 3s
Show
Sleep 1s

# Open the Code note (5 down: 4 folders, Basalt, then Code)
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

# Focus the editor and hide the explorer so the code block gets the width
Tab
Sleep 1s
Ctrl+B
Sleep 2.5s

# Enter edit mode and walk down into the code block. The cursor's line shows
# its raw text but keeps the highlighting.
Type "i"
Sleep 1.5s
Down
Sleep 0.3s
Down
Sleep 0.3s
Down
Sleep 0.3s
Down
Sleep 0.3s
Down
Sleep 0.9s
Down
Sleep 0.9s
Down
Sleep 0.9s
Down
Sleep 0.9s
Down
Sleep 0.9s
Down
Sleep 0.9s
Down
Sleep 0.9s
Down
Sleep 1.5s

# Open a blank line above `println!` and type new code. The colours follow
# every keystroke.
Enter
Sleep 0.5s
Up
Sleep 0.8s
Type@120ms "    // Shout it"
Sleep 0.8s
Enter
Sleep 0.5s
Type@120ms "    let loud = greeting.to_uppercase();"
Sleep 2s
Escape
Sleep 2s

# Switch to read view to see the block fully rendered
Shift+R
Sleep 3s

# Quit (without saving)
Type "q"

# Cleanup
Hide
Type "rm docs/Code.md"
Enter
