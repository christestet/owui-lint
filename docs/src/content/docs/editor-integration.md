---
title: Editor Integration
description: Use owui-lint as an LSP server.
---

`owui-lint server` starts the language server over stdio for editor clients.

```bash
owui-lint server
```

The server provides Open WebUI-specific intelligence on top of a general Python language server (Pylance/Pyright); it does not replace general Python tooling. It identifies itself to the client via `serverInfo` (`owui-lint` plus the binary version).

- **Diagnostics** — the same rule engine used by the CLI, offered both ways:
  - *Push* — published on open/change and cleared on close (`textDocument/publishDiagnostics`).
  - *Pull* — computed on demand via the LSP 3.17 pull model (`textDocument/diagnostic`); clients that prefer pull use this and ignore the push. After a project-wide change such as disabling a rule, the server asks pull-model clients to re-query (`workspace/diagnostic/refresh`).
- **Hover** — on a finding line: rule title, summary, **Fix:** remediation, a `[Documentation](url)` link, and the minimum Open WebUI version required.
- **Completions** — Open WebUI scaffolding snippets for Python files, selected by cursor context:
  - *Inside the module docstring* — header field snippets: `version`, `title`, `requirements`, `author`, `description`.
  - *At top level (no indentation)* — class skeleton snippets: `owui-tools`, `owui-pipe`, `owui-filter`, `owui-action`, `owui-pipeline`.
  - *Inside a class body (indented)* — class member snippets: `Valves`, `UserValves`, `pipe-method`, `inlet-method`, `outlet-method`, `action-method`.
- **Quick fixes** — make a handler `async` (OWT102, OWP202, OWA401), add a docstring stub (OWT101), add a missing module-header field (OWUI030 `version`, OWUI032 `title`), or disable a rule for the project.
- **Logging** — operational events (startup, per-file lint summaries, recoverable errors) are sent via `window/logMessage` to the editor's output channel, filtered by the client's log/trace level.

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
- **owui-lint: Show Output** (`owui-lint.showOutput`) — reveals the server's output channel, where `window/logMessage` entries and (when `owui-lint.trace.server` is `messages`/`verbose`) the LSP message trace appear.

### Troubleshooting

- For a closer look at what the server is doing, run **owui-lint: Show Output** and set `owui-lint.trace.server` to `messages` or `verbose`.
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

Any editor with a generic LSP client can run owui-lint over stdio. The recipe is
always the same:

- **Command:** `owui-lint server`
- **File types:** `python`
- **Root markers:** `config.yml`, `owui-lint.yml` (or `.git`) — the server reads the workspace [configuration](configuration/) from here.

No `initializationOptions` are required. owui-lint is meant to run **alongside** a
general Python language server (Pyright, Pylsp, Jedi); enable both wherever the
client supports multiple servers for one file type.

### Neovim (built-in, 0.11+)

No plugin needed. In your `init.lua`:

```lua
vim.lsp.config("owui_lint", {
  cmd = { "owui-lint", "server" },
  filetypes = { "python" },
  root_markers = { "config.yml", "owui-lint.yml", ".git" },
})
vim.lsp.enable("owui_lint")
```

### Neovim (nvim-lspconfig)

owui-lint is not in the lspconfig registry, so register it as a custom server:

```lua
local lspconfig = require("lspconfig")
local configs = require("lspconfig.configs")

if not configs.owui_lint then
  configs.owui_lint = {
    default_config = {
      cmd = { "owui-lint", "server" },
      filetypes = { "python" },
      root_dir = lspconfig.util.root_pattern("config.yml", "owui-lint.yml", ".git"),
      single_file_support = true,
    },
  }
end

lspconfig.owui_lint.setup({})
```

### Vim (vim-lsp)

```vim
if executable('owui-lint')
  augroup owui_lint
    autocmd!
    autocmd User lsp_setup call lsp#register_server({
          \ 'name': 'owui-lint',
          \ 'cmd': {server_info->['owui-lint', 'server']},
          \ 'allowlist': ['python'],
          \ })
  augroup END
endif
```

### Vim/Neovim (coc.nvim)

In `coc-settings.json`:

```jsonc
{
  "languageserver": {
    "owui-lint": {
      "command": "owui-lint",
      "args": ["server"],
      "filetypes": ["python"],
      "rootPatterns": ["config.yml", "owui-lint.yml", ".git"]
    }
  }
}
```

### Helix

In `languages.toml` (keep your existing Python server in the list):

```toml
[language-server.owui-lint]
command = "owui-lint"
args = ["server"]

[[language]]
name = "python"
language-servers = ["pyright", "owui-lint"]
```

### Sublime Text (LSP package)

In `LSP.sublime-settings`:

```jsonc
{
  "clients": {
    "owui-lint": {
      "enabled": true,
      "command": ["owui-lint", "server"],
      "selector": "source.python"
    }
  }
}
```

### Emacs (Eglot)

```elisp
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '((python-mode python-ts-mode) . ("owui-lint" "server"))))
```

Eglot runs a single server per buffer, so this **replaces** your Python server.
To run owui-lint and Pyright together, use `lsp-mode`, which supports multiple
servers for one major mode.

### Verifying

These clients drive diagnostics over the push model
(`textDocument/publishDiagnostics`) or the LSP 3.17 pull model
(`textDocument/diagnostic`) — owui-lint supports both, so use whichever your
client prefers. To confirm the server is running, open a Python file with an
Open WebUI extension and check that `OWUI*`/`OWT*`/`OWP*`/`OWF*`/`OWA*`/`OWPL*`
findings appear; if not, run `owui-lint path/to/file.py` in a terminal to verify
the binary and your configuration.
