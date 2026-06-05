---
title: Editor Integration
description: Use owui-lint as an LSP server.
---

`owui-lint server` starts the language server over stdio for editor clients.

```bash
owui-lint server
```

The server provides Open WebUI-specific intelligence on top of a general Python language server (Pylance/Pyright); it does not replace general Python tooling.

- **Diagnostics** — the same rule engine used by the CLI; published on open/change, cleared on close.
- **Hover** — on a finding line: rule title, summary, **Fix:** remediation, a `[Documentation](url)` link, and the minimum Open WebUI version required.
- **Completions** — Open WebUI scaffolding snippets for Python files, selected by cursor context:
  - *Inside the module docstring* — header field snippets: `version`, `title`, `requirements`, `author`, `description`.
  - *At top level (no indentation)* — class skeleton snippets: `owui-tools`, `owui-pipe`, `owui-filter`, `owui-action`, `owui-pipeline`.
  - *Inside a class body (indented)* — class member snippets: `Valves`, `UserValves`, `pipe-method`, `inlet-method`, `outlet-method`, `action-method`.
- **Quick fixes** — make a handler `async` (OWT102, OWP202, OWA401), add a docstring stub (OWT101), add a missing module-header field (OWUI030 `version`, OWUI032 `title`), or disable a rule for the project.

## VS Code Extension

The repository includes a VS Code extension under `editors/vscode/`. It activates on Python files (`onLanguage:python`) and launches `owui-lint server` over stdio.

```bash
npm install --prefix editors/vscode
npm run compile --prefix editors/vscode
```

Press <kbd>F5</kbd> in VS Code to launch an Extension Development Host.

### Settings

| Setting | Default | Description |
|---|---|---|
| `owui-lint.path` | `owui-lint` | Path to the `owui-lint` executable. |
| `owui-lint.trace.server` | `off` | LSP trace level: `off`, `messages`, or `verbose`. |

### Commands

- **owui-lint: Restart Language Server** (`owui-lint.restartServer`) — stops and restarts the language server process without reloading VS Code.

## Other Editors

Any editor with a generic LSP client (Neovim, Helix, Zed, etc.) can run the server over stdio. Configure the command as:

```text
owui-lint server
```

Diagnostics use the workspace configuration discovered by the CLI config loader.
