#!/bin/sh
set -eu

REPO="christestet/owui-lint"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

detect_target() {
  arch=$(uname -m)
  os=$(uname -s)
  case "$os" in
    Linux)
      case "$arch" in
        x86_64)  echo "x86_64-unknown-linux-gnu" ;;
        aarch64) echo "aarch64-unknown-linux-gnu" ;;
        *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        x86_64)  echo "x86_64-apple-darwin" ;;
        arm64)   echo "aarch64-apple-darwin" ;;
        *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
      esac
      ;;
    *) echo "Unsupported OS: $os" >&2; exit 1 ;;
  esac
}

TARGET=$(detect_target)
ARCHIVE="owui-lint-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${ARCHIVE}"

echo "Detected target: ${TARGET}"
echo "Downloading ${URL}..."

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

curl --proto '=https' --tlsv1.2 -LsSf "$URL" -o "${tmpdir}/${ARCHIVE}"
tar xzf "${tmpdir}/${ARCHIVE}" -C "${tmpdir}"

install -d "${INSTALL_DIR}"
install -m 755 "${tmpdir}/owui-lint" "${INSTALL_DIR}/owui-lint"

echo "Installed owui-lint to ${INSTALL_DIR}/owui-lint"

case ":${PATH}:" in
  *:"${INSTALL_DIR}":*) ;;
  *) echo "Warning: ${INSTALL_DIR} is not in your PATH. Add it to your shell profile." ;;
esac
