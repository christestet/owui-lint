---
title: Configuration
description: Configure file discovery and rule severity.
---

`owui-lint` looks for a config file by searching the current directory (CLI) or the workspace root (LSP server) in this exact order:

1. `config.yml`
2. `config.yaml`
3. `owui-lint.yml`
4. `owui-lint.yaml`

The first file found is used; the rest are ignored. Pass `--config <PATH>` to force a specific file — if that path does not exist the command exits with code 2.

> **Parser note:** `owui-lint` uses a lightweight, hand-rolled YAML subset. Only the keys documented here (`lint.include`, `lint.exclude`, and `rules.*`) are read. Full YAML features such as anchors, aliases, and nested maps are not supported.

```yaml
lint:
  include:
    - "**/*.py"
  exclude:
    - ".git/**"
    - ".venv/**"
    - "**/__pycache__/**"

rules:
  OWT101: error
  OWP202: warning
  OWUI020: off
```

## Include And Exclude

`include` controls which paths are eligible for linting. `exclude` removes paths from that set. If `include` is set to an empty list it falls back to the default `**/*.py`. Directory targets are still required on the command line:

```bash
owui-lint extensions/ --config owui-lint.yml
```

**Defaults**

| Key | Default |
|-----|---------|
| `lint.include` | `["**/*.py"]` |
| `lint.exclude` | `[".git/**", ".venv/**", "**/__pycache__/**"]` |

## Rule Overrides

Each rule has a default severity. Override it in the `rules:` section using the rule ID (case-insensitive) and one of these values:

- `error` — report as an error; non-zero exit by default
- `warning` — report as a warning
- `off` — disable the rule entirely

```yaml
rules:
  OWT101: error
  OWP202: warning
  OWUI020: off
```

Unknown rule IDs in the config file are reported as warnings so configuration drift is visible without blocking the run. When this happens the CLI suggests running `owui-lint rules` to list the valid IDs.
