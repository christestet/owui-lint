---
title: Configuration
description: Configure file discovery and rule severity.
---

`owui-lint` looks for `config.yml`, `owui-lint.yml`, or `owui-lint.yaml` by default. Pass `--config <PATH>` to use a specific file.

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

`include` controls which paths are eligible for linting. `exclude` removes paths from that set. Directory targets are still required on the command line:

```bash
owui-lint extensions/ --config owui-lint.yml
```

## Rule Overrides

Each rule has a default severity. Override it with:

- `error`
- `warning`
- `off`

Unknown rule IDs are reported as warnings so configuration drift is visible without blocking every run.
