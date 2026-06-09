#!/usr/bin/env bash
# Single source of truth for all icon/image assets lives in assets/.
# This mirrors the master assets into every location a build or registry
# needs its own copy, so you only ever edit files under assets/.
#
#   sync-assets.sh            # write copies (default)
#   sync-assets.sh --write    # write copies
#   sync-assets.sh --check    # fail if any copy is out of sync (CI)
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# master:destination pairs (edit only the master, never the destination)
mappings=(
  "assets/icon.svg:docs/public/owui-lint-icon.svg"
  "assets/icon.svg:docs/src/assets/owui-lint-icon.svg"
  "assets/icon-128.png:editors/vscode/icon.png"
)

mode="write"
case "${1:-}" in
  --check) mode="check" ;;
  --write | "") mode="write" ;;
  *)
    echo "usage: $0 [--write|--check]" >&2
    exit 2
    ;;
esac

status=0
for pair in "${mappings[@]}"; do
  src="${pair%%:*}"
  dst="${pair#*:}"

  if [[ ! -f "$src" ]]; then
    echo "error: missing master asset: $src" >&2
    exit 1
  fi

  if [[ "$mode" == "check" ]]; then
    if ! cmp -s "$src" "$dst" 2>/dev/null; then
      echo "out of sync: $dst" >&2
      status=1
    fi
  else
    if cmp -s "$src" "$dst" 2>/dev/null; then
      continue
    fi
    mkdir -p "$(dirname "$dst")"
    cp "$src" "$dst"
    echo "synced: $src -> $dst"
  fi
done

if [[ "$mode" == "check" && "$status" -ne 0 ]]; then
  echo "assets are out of sync; run 'make assets-sync' and commit the result" >&2
fi

exit "$status"
