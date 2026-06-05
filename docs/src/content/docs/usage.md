---
title: Usage
description: Common owui-lint commands and CI output modes.
---

Run against one or more files or directories:

```bash
owui-lint path/to/extensions
owui-lint tool.py filter.py
```

Directories are scanned recursively for Python files using the active include and exclude patterns.

## Output Formats

```bash
owui-lint extensions/ --format text
owui-lint extensions/ --format json --output owui-lint.json
owui-lint extensions/ --format github
owui-lint extensions/ --format sarif --output owui-lint.sarif
```

For GitHub Actions annotations and SARIF code scanning, see [CI Setup](ci/).

## Exit Behavior

```bash
owui-lint extensions/ --fail-on error
owui-lint extensions/ --fail-on warning
owui-lint extensions/ --fail-on none
```

Exit codes:

- `0`: no configured failure condition was met
- `1`: the `--fail-on` condition was met (errors found with `--fail-on error`; any findings with `--fail-on warning`)
- `2`: usage, configuration, or runtime error (no targets given, no files matched, write failure)

## Rule Discovery

```bash
owui-lint rules
owui-lint explain OWT101
```

`owui-lint rules --format json` emits a machine-readable rule list.

## Self-Update

```bash
owui-lint update
```

Checks for a newer release and installs it in place. Returns `0` when already up to date or after a successful install; `1` if the installer fails.

The generated [CLI reference](../reference/cli/) lists every command from live `--help` output.
