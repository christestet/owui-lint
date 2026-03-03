# AGENTS.md

## Why

`owui-lint` is a standalone CLI linter that catches mistakes in Open WebUI extensions (Tools, Pipes, Filters, Actions, Pipelines) before they reach production.

## What

- **Language:** Rust (single binary, no runtime dependencies)
- **Config:** `owui-lint.yml` (see `owui-lint.example.yml` for schema)
- **Source layout:** `src/` — `cli.rs` (arg parsing), `analysis/` (AST walks: `mod.rs` orchestration, `syntax.rs` error detection, `parsing.rs` Python construct parsing), `rules.rs` (lint rules), `linter.rs` (orchestration), `output.rs` (text/JSON/SARIF reporters), `config.rs`, `models.rs`
- **Tests:** `tests/` (integration), inline `#[cfg(test)]` (unit)

## How

Use the Makefile for all common workflows:

```
make check        # fmt-check + clippy + test (the CI gate)
make run          # lint current directory
make build        # debug build
```

Verify every change with `make check` before committing.

## Deep-Dive Docs

Read these **on demand** when the task requires domain knowledge — do not load them all upfront.

| Topic | Path | When to read |
|---|---|---|
| Open WebUI extension specs & source | `.agents/openwebui-extensions/SKILL.md` | Adding/modifying lint rules for extension types. Includes markdown references AND Python source code from the Open WebUI backend (`plugin.py`, `tools.py`, `filter.py`, `actions.py`) for exact function signatures and runtime behavior. |
| Rust anti-patterns | `.agents/skills/rust-antipatterns/SKILL.md` | Writing or reviewing Rust implementation code |
