# owui-lint for VS Code

Live linting for [Open WebUI](https://docs.openwebui.com/) extensions — Tools,
Pipe, Filter, Action, Event and Pipeline — powered by the `owui-lint` language
server.

This extension **complements** a Python language server (Pylance/Pyright). It
does not provide general Python IntelliSense or syntax checking; it adds only the
Open WebUI–specific layer:

- **Diagnostics** while you type (the `OWT*/OWP*/OWF*/OWA*/OWE*/OWPL*/OWUI*` rules).
- **Hover** explanations with remediation and docs links for each finding.
- **Quick fixes**: make a handler `async`, add a docstring stub, add missing
  module-header fields, or disable a rule for the project.
- **Completions** — live LSP scaffolding snippets for Open WebUI Python files:
  class skeletons (`owui-tools`, `owui-pipe`, `owui-filter`, `owui-action`,
  `owui-event`, `owui-pipeline`), `Valves`/`UserValves`, handler methods
  (`pipe-method`, `inlet-method`, `outlet-method`, `action-method`,
  `event-method`), and module header fields
  (`version`, `title`, `requirements`, `author`, `description`).

## Requirements

The `owui-lint` binary must be installed and on your `PATH`, or its location set
via the `owui-lint.path` setting.

```jsonc
{
  "owui-lint.path": "/absolute/path/to/owui-lint"
}
```

For local development in this repository, build the binary and point the
extension at the workspace-local executable:

```jsonc
{
  "owui-lint.path": "target/debug/owui-lint"
}
```

### Settings

| Setting | Default | Description |
|---|---|---|
| `owui-lint.path` | `owui-lint` | Path to the `owui-lint` executable. Supports commands on `PATH`, absolute paths, workspace-relative paths, and `${workspaceFolder}`. |
| `owui-lint.trace.server` | `off` | LSP trace level: `off`, `messages`, or `verbose`. |

### Commands

- **owui-lint: Restart Language Server** (`owui-lint.restartServer`) — restarts the language server without reloading VS Code.

## Develop / run locally

```bash
cd editors/vscode
npm ci
npm run compile
```

Then press <kbd>F5</kbd> in VS Code to launch an Extension Development Host. Open a
Python file containing an Open WebUI extension to see diagnostics.

## Package a `.vsix`

```bash
npm ci
npm run vsix
```

## Other editors

The server speaks standard LSP over stdio (`owui-lint server`). Editors such as
Zed, Neovim or Helix can launch it directly via their generic LSP configuration.
