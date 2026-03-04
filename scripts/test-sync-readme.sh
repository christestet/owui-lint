#!/usr/bin/env bash
# Test suite for docs-sync Rust binary
set -euo pipefail

passes=0
fails=0
total=0

pass() {
  ((passes++)) || true
  ((total++)) || true
  echo "  PASS: $1"
}

fail() {
  ((fails++)) || true
  ((total++)) || true
  echo "  FAIL: $1"
}

assert_zero() {
  local label="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    pass "$label"
  else
    fail "$label - expected zero exit"
  fi
}

assert_nonzero() {
  local label="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    fail "$label - expected non-zero exit"
  else
    pass "$label"
  fi
}

if ! command -v cargo >/dev/null 2>&1; then
  echo "Skipping docs-sync tests (cargo not available)."
  exit 0
fi

README_FILE="README.md"
BACKUP="$(mktemp)"
cp "$README_FILE" "$BACKUP"
cleanup() {
  cp "$BACKUP" "$README_FILE"
  rm -f "$BACKUP"
}
trap cleanup EXIT

run_sync() {
  cargo run --locked --bin docs-sync -- "$@"
}

inject_drift() {
  local tmp
  tmp="$(mktemp)"
  local in_block=0
  while IFS= read -r line; do
    if [[ "$line" == "<!-- BEGIN:OWUI_LINT_COMMANDS -->" ]]; then
      echo "$line" >> "$tmp"
      echo "MANUAL-DRIFT" >> "$tmp"
      in_block=1
      continue
    fi
    if [[ "$in_block" -eq 1 ]]; then
      if [[ "$line" == "<!-- END:OWUI_LINT_COMMANDS -->" ]]; then
        echo "$line" >> "$tmp"
        in_block=0
      fi
      continue
    fi
    echo "$line" >> "$tmp"
  done < "$README_FILE"
  mv "$tmp" "$README_FILE"
}

echo "=== Test Suite: docs-sync ==="
echo ""

assert_zero "Writes generated sections" run_sync --write
assert_zero "Check passes after write" run_sync --check

inject_drift
assert_nonzero "Check fails on drift" run_sync --check
assert_zero "Write restores generated content" run_sync --write
assert_zero "Check passes after restore" run_sync --check

echo ""
echo "=== Results: ${passes} passed, ${fails} failed, ${total} total ==="
if [[ "$fails" -gt 0 ]]; then
  exit 1
fi
