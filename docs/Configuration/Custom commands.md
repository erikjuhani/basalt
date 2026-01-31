You can execute external commands using special command prefixes in your [[Key mappings|key bindings]].

## Command types

### Execute command (`exec:`)

Runs a command in the current shell environment. Blocks until completion. Only the first argument is treated as the executable, with remaining arguments passed as literal parameters.

```toml
key_bindings = [
  { key = "ctrl+alt+e", command = "exec:nvim %note_path" },
]
```

### Spawn process (`spawn:`)

Spawns a new process without blocking. Use this for opening external applications or URLs.

```toml
key_bindings = [
  { key = "ctrl+o", command = "spawn:open %note" },
  { key = "ctrl+b", command = "spawn:open https://example.com" },
]
```

## Variables

Variables are dynamically replaced with current context information at runtime.

| Variable | Description | Example |
| --- | --- | --- |
| `%vault` | Current vault name | `my-notes` |
| `%note` | Current note name | `My Note` |
| `%note_path` | Current note file path | `/path/to/vault/daily/2024-01-15.md` |

## Platform considerations

- **macOS**: Use `open` for launching applications
- **Linux**: Use `xdg-open` for opening files/URLs with default applications
- **Windows**: Use `start` for opening files/URLs

## Integration examples

### Obsidian

Obsidian supports URL schemes starting with `obsidian://`, which can be used with `spawn:`:

```toml
key_bindings = [
  # Open daily note in Obsidian
  { key = "ctrl+d", command = "spawn:open obsidian://daily?vault=%vault" },

  # Open current note in Obsidian
  { key = "ctrl+alt+e", command = "spawn:open obsidian://open?vault=%vault&file=%note" },

  # Create new note in Obsidian
  { key = "ctrl+n", command = "spawn:open obsidian://new?vault=%vault&name=New Note" },

  # Open specific path in Obsidian
  { key = "ctrl+p", command = "spawn:open obsidian://open?path=%note_path" },
]
```

Common Obsidian URI actions: `obsidian://open`, `obsidian://new`, `obsidian://daily`, `obsidian://search`. For more information, see the [Obsidian URI documentation](https://help.obsidian.md/Extending+Obsidian/Obsidian+URI).

### External editors

```toml
key_bindings = [
  # Open in VS Code
  { key = "ctrl+c", command = "spawn:code %note_path" },

  # Open in vim (blocking)
  { key = "ctrl+v", command = "exec:vim %note_path" },

  # Open in nano (blocking)
  { key = "ctrl+n", command = "exec:nano %note_path" },
]
```

### Web

```toml
key_bindings = [
  # Open a URL
  { key = "ctrl+shift+g", command = "spawn:open https://github.com/user/repo" },

  # Search with note name
  { key = "ctrl+s", command = "spawn:open https://www.google.com/search?q=%note" },
]
```

> [!NOTE]
>
> These are examples — choose key combinations that don't conflict with your existing bindings. Check the [[Configuration|default configuration]] for keys already in use.

## Tips

- Use `spawn:` for non-blocking operations like opening applications or URLs
- Use `exec:` for commands that should complete before continuing
- Shell features like pipes (`|`), redirects (`>`), and command substitution (`$(...)`) are not supported
- Wrap complex operations in scripts that can be called as single commands
- Ensure external commands are available in your `PATH`
- Variables require the relevant context to be active (e.g., `%note` needs a note to be selected)
