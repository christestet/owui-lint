#!/usr/bin/env bash
set -euo pipefail

HOOK_PATH=".git/hooks/pre-commit"

if [[ ! -d .git ]]; then
  echo "Run this script from the repository root." >&2
  exit 1
fi

cat > "$HOOK_PATH" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

cargo run --locked --bin docs-sync -- --write
git add README.md
EOF

chmod +x "$HOOK_PATH"
echo "Installed pre-commit hook at $HOOK_PATH"
