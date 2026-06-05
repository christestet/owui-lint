---
title: Editor Integration
description: Use owui-lint as an LSP server.
---

`owui-lint server` starts the language server over stdio for editor clients.

```bash
owui-lint server
```

The server provides:

- diagnostics from the same rule engine used by the CLI
- hover details for known rule IDs
- completions for rule IDs in configuration files
- quick fixes for supported findings

## VS Code Extension

The repository includes a VS Code extension under `editors/vscode/`.

```bash
npm ci --prefix editors/vscode
npm run compile --prefix editors/vscode
```

The extension expects an `owui-lint` binary on `PATH` or a configured binary path, then launches `owui-lint server` for workspace feedback.

## Other Editors

Any editor with a generic LSP client can run the binary over stdio. Configure the command as:

```text
owui-lint server
```

Diagnostics use the workspace configuration discovered by the CLI config loader.
