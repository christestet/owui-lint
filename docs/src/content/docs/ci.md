---
title: CI Setup
description: Run owui-lint in GitHub Actions and publish SARIF results.
---

Use `owui-lint` in CI to fail pull requests on Open WebUI extension issues and, optionally, publish SARIF results to GitHub code scanning.

## Basic GitHub Actions Job

Install the latest release binary, then run the linter against the directory that contains your Open WebUI extensions:

```yaml
name: owui-lint

on:
  pull_request:
  push:
    branches: [main]

jobs:
  lint-open-webui-extensions:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v5

      - name: Install owui-lint
        run: curl -fsSL https://raw.githubusercontent.com/christestet/owui-lint/main/scripts/install.sh | sh

      - name: Run owui-lint
        run: owui-lint path/to/extensions --fail-on error
```

Use `--fail-on warning` if warnings should block the workflow, or `--fail-on none` when you only want reports without a failing job.

## GitHub Annotations

The `github` output format emits workflow command annotations that appear inline in the Actions log and pull request UI:

```yaml
- name: Run owui-lint with annotations
  run: owui-lint path/to/extensions --format github --fail-on warning
```

This is the simplest CI feedback mode because it does not require uploading an artifact.

## SARIF Code Scanning

For repository-level code scanning alerts, emit SARIF and upload it with GitHub's SARIF action:

```yaml
- name: Run owui-lint SARIF
  run: owui-lint path/to/extensions --format sarif --output owui-lint.sarif --fail-on none

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: owui-lint.sarif
```

`--fail-on none` is useful for SARIF-only jobs because GitHub can still ingest the report even when findings exist. If you also want CI to block on findings, use a separate `owui-lint path/to/extensions --fail-on error` step or change the SARIF step's exit policy.

## Configuration

Commit an `owui-lint.yml` file at the repository root to keep local runs, editor diagnostics, and CI behavior aligned:

```yaml
lint:
  include:
    - "extensions/**/*.py"
  exclude:
    - ".venv/**"
    - "**/__pycache__/**"

rules:
  OWT101: error
  OWUI020: off
```

See [Configuration](configuration/) for the full supported schema.
