#!/bin/sh
set -eu

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BINARY="${INSTALL_DIR}/owui-lint"

if [ ! -f "$BINARY" ]; then
  # Fall back to whichever owui-lint is on PATH
  BINARY=$(command -v owui-lint 2>/dev/null || true)
  if [ -z "$BINARY" ]; then
    echo "owui-lint not found." >&2
    exit 1
  fi
fi

rm -f "$BINARY"
echo "Removed $BINARY"
