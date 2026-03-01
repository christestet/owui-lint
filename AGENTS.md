# AGENTS.md

## Open WebUI Lint - owui-lint

`owui-lint` is a Rust-only CLI linter for Open WebUI extensions:

- Tools
- Pipe
- Filter
- Action
- Pipeline

## Goal

- Distribute as a single binary CLI tool.
- Configure linting rules via `config.yml`/`owui-lint.yml`.
- Use Makefile

## Docs

For Open WebUI extension specifics, use references in:

- `.agents/openwebui-extensions/SKILL.md`

## Project

- Rust
- CLI, analysis, rule engine, and output all implemented in Rust
- Binary distribution via `cargo build --release`

## Testing and Quality

- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo test`
