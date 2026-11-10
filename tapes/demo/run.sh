#!/bin/sh
# Runs basalt inside a captioned tmux session with a throwaway HOME, so the
# recorder's own config and Obsidian vaults never leak into the demo.
set -eu

demo=$(cd "$(dirname "$0")" && pwd)
home=$(mktemp -d)
note="docs/Columnar Basalt.md"
trap 'rm -rf "$home" "$note"' EXIT

for dir in "$home/Library/Application Support/obsidian" "$home/.config/obsidian"; do
  mkdir -p "$dir"
  echo '{"vaults":{}}' > "$dir/obsidian.json"
done
cp "$demo/basalt.toml" "$home/.basalt.toml"
cp "$demo/Columnar Basalt.md" "$note"

DEMO_DIR=$demo tmux -u -L "basalt-demo-$$" -f "$demo/tmux.conf" new-session \
  "HOME='$home' BASALT_EXP_VAULT_PATH=docs target/release/basalt --theme ${BASALT_THEME:-causeway-dark}"
