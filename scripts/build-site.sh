#!/usr/bin/env bash
# Build or serve the docs site.
#
#   ./scripts/build-site.sh           # one-shot build
#   ./scripts/build-site.sh serve     # zola serve with live reload
#   ./scripts/build-site.sh build --strict
#
# The transform runs once before any Zola invocation. For `serve`, re-run
# the script (or wire up watchexec/entr on `docs/`) to pick up source edits.

set -euo pipefail

cd "$(dirname "$0")/.."

cargo run -p obsidian-to-zola --release -- ${TRANSFORM_FLAGS:-}

cmd="${1:-build}"
shift || true

case "$cmd" in
  build|serve|check)
    exec zola --root site "$cmd" "$@"
    ;;
  *)
    echo "usage: $0 [build|serve|check] [zola args...]" >&2
    exit 2
    ;;
esac
