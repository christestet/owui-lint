# owui-lint for VS Code

Live linting for [Open WebUI](https://docs.openwebui.com/) extensions — Tools,
Pipe, Filter, Action and Pipeline — powered by the `owui-lint` language server.

This extension **complements** a Python language server (Pylance/Pyright). It
does not provide general Python IntelliSense or syntax checking; it adds only the
Open WebUI–specific layer:

- **Diagnostics** while you type (the `OWT*/OWP*/OWF*/OWA*/OWPL*/OWUI*` rules).
- **Hover** explanations with remediation and docs links for each finding.
- **Quick fixes**: make a handler `async`, add a docstring stub, add missing
  module-header fields, or disable a rule for the project.
- **Snippets** for Open WebUI scaffolding: class skeletons (`owui-tools`,
  `owui-pipe`, …), `Valves`/`UserValves`, handler methods, and header fields.

## Requirements

The `owui-lint` binary must be installed and on your `PATH`, or its location set
via the `owui-lint.path` setting.

```jsonc
{
  "owui-lint.path": "/absolute/path/to/owui-lint"
}
```

## Develop / run locally

```bash
cd editors/vscode
npm install
npm run compile
```

Then press <kbd>F5</kbd> in VS Code to launch an Extension Development Host. Open a
Python file containing an Open WebUI extension to see diagnostics.

## Package a `.vsix`

```bash
npm install -g @vscode/vsce
vsce package
```

## Other editors

The server speaks standard LSP over stdio (`owui-lint server`). Editors such as
Zed, Neovim or Helix can launch it directly via their generic LSP configuration.
