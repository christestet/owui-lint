---
title: Install
description: Install owui-lint from releases, source, or Docker.
---

## Pre-built Binaries

The recommended way to install owui-lint. Binaries are published for macOS (aarch64, x86_64), Linux (aarch64, x86_64), and Windows (aarch64, x86_64).

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

Requires Rust 1.93.1 or later (MSRV).

```bash
cargo build --release
./target/release/owui-lint --help
```

## Docker

No local Rust or Cargo required. `docker-install` copies the binary out of the image into `INSTALL_DIR`.

```bash
make docker-build
make docker-run TARGET=.
make docker-install INSTALL_DIR="$HOME/.local/bin"
```

## Updating

Once installed, use the built-in self-update command to fetch and install the latest release:

```bash
owui-lint update
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
