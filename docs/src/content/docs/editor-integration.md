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

The VS Code extension activates on Python files (`onLanguage:python`) and launches `owui-lint server` over stdio. It does not bundle the CLI binary, so install `owui-lint` first.

### Install

The extension is packaged as a `.vsix` release asset:

1. Download the `.vsix` from the [latest release](https://github.com/christestet/owui-lint/releases/latest).
2. In VS Code, run **Extensions: Install from VSIX...**.
3. Open a Python file containing an Open WebUI extension.

If VS Code cannot find the binary, set `owui-lint.path` to an absolute path.

### Features

- Live diagnostics for the same `OWUI*`, `OWT*`, `OWP*`, `OWF*`, `OWA*`, and `OWPL*` rules used by the CLI.
- Hover documentation on finding lines, including the rule summary, remediation, documentation link, and minimum Open WebUI version.
- Completion snippets for module headers, extension class skeletons, `Valves`/`UserValves`, and common handler methods.
- Quick fixes for async handlers, missing tool docstrings, missing `title`/`version` header fields, and project-wide rule disables.

### Settings

| Setting | Default | Description |
|---|---|---|
| `owui-lint.path` | `owui-lint` | Path to the `owui-lint` executable. |
| `owui-lint.trace.server` | `off` | LSP trace level: `off`, `messages`, or `verbose`. |

### Commands

- **owui-lint: Restart Language Server** (`owui-lint.restartServer`) — stops and restarts the language server process without reloading VS Code.

### Troubleshooting

- If no diagnostics appear, confirm the file language mode is Python and run `owui-lint path/to/file.py` in a terminal.
- If the server cannot start, set `owui-lint.path` to the absolute binary path and run **owui-lint: Restart Language Server**.
- If Python syntax errors are missing, enable a Python language server such as Pylance or Pyright; `owui-lint` only reports Open WebUI-specific issues.

## Extension Development

The extension source lives under `editors/vscode/`.

```bash
npm install --prefix editors/vscode
npm run compile --prefix editors/vscode
```

Press <kbd>F5</kbd> in VS Code to launch an Extension Development Host.

## Other Editors

Any editor with a generic LSP client (Neovim, Helix, Zed, etc.) can run the server over stdio. Configure the command as:

```text
owui-lint server
```

Diagnostics use the workspace configuration discovered by the CLI config loader.
