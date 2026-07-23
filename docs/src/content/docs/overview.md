---
title: Overview
description: What owui-lint checks and where it fits.
---

`owui-lint` is a standalone Rust CLI for checking Open WebUI extension files before they are shared, committed, or deployed.

It focuses on Open WebUI-specific structure and runtime compatibility:

- extension class detection for Tools, Pipe, Filter, Action, Event, and Pipeline files
- Valves and UserValves configuration patterns
- required hook methods and payload parameters
- public tool method quality
- module metadata such as `title`, `version`, and `requirements`

The Python analysis is intentionally lightweight. `OWUI001` catches basic delimiter and string issues, but `owui-lint` is not a full Python parser. Use `python -m py_compile <file.py>` alongside it when you need full Python syntax validation.

## Configuration

Config is loaded automatically from `config.yml`, `config.yaml`, `owui-lint.yml`, or `owui-lint.yaml` in the working directory. You can also pass `-c <path>` explicitly. Rules can be turned off or promoted to errors in the `rules:` section; file discovery is controlled by `lint.include` and `lint.exclude` glob patterns.

Default include pattern: `**/*.py`. Default excludes: `.git/**`, `.venv/**`, `**/__pycache__/**`.
