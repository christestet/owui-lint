---
title: Install
description: Install owui-lint from releases, source, or Docker.
---

## Pre-built Binaries

macOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/christestet/owui-lint/main/scripts/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/christestet/owui-lint/main/scripts/install.ps1 | iex
```

You can also download a binary from the [latest release](https://github.com/christestet/owui-lint/releases/latest).

## Build From Source

```bash
cargo build --release
./target/release/owui-lint --help
```

## Docker

```bash
make docker-build
make docker-run TARGET=.
make docker-install INSTALL_DIR="$HOME/.local/bin"
```

## Uninstall

macOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/christestet/owui-lint/main/scripts/uninstall.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/christestet/owui-lint/main/scripts/uninstall.ps1 | iex
```
