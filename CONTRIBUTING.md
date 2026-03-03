# Contributing to owui-lint

This project is intentionally structured so adding a new lint rule is a small, repeatable change.

## Rule ID Schema

Every rule ID follows the pattern `OW<PREFIX><NNN>`:

| Prefix | Extension type | Number range | Example |
|--------|---------------|-------------|---------|
| `OWUI` | Universal (any extension) | 001–099 | `OWUI022` |
| `OWT` | Tools | 100–199 | `OWT101` |
| `OWP` | Pipe | 200–299 | `OWP202` |
| `OWF` | Filter | 300–399 | `OWF301` |
| `OWA` | Action | 400–499 | `OWA401` |
| `OWPL` | Pipeline | 500–599 | `OWPL501` |

Pick the next available number in the range that matches the scope of your rule. Use `OWUI` for checks that apply regardless of extension type (e.g., header metadata, Valves).

## RuleDoc Field Guide

Each rule is declared once in `src/rules.rs` as a `RuleDoc`. The five string fields power every output format (`text`, `json`, `github`, `sarif`) and the `rules`/`explain` sub-commands.

```rust
RuleDoc {
    id: OWUIXXX,                   // rule ID constant (must match the const name)
    default_severity: Severity::Warning,
    title: "...",                  // short noun phrase, ≤ 50 chars, no trailing punctuation
    summary: "...",                // one sentence describing what is wrong, ends with `.`
    remediation: "...",            // one sentence telling users exactly what to do, ends with `.`
    help_url: SOME_CONST,          // link to Open WebUI docs for background
}
```

### Writing good field values

**`title`** — A short noun phrase that names the problem. Scanned in tables and CI annotations.
- Good: `"Missing Valves class"`, `"Pipe method should be async"`
- Avoid: `"The Valves class is missing"`, `"You should make pipe async"`

**`summary`** — One sentence (ends with `.`) describing *what is wrong* and *why it matters*.
- Good: `"Extensions should provide a nested \`Valves\` class for runtime configuration."`
- Avoid repeating the title verbatim or giving the fix here.

**`remediation`** — One sentence (ends with `.`) telling users *exactly* what to change. Use imperative voice. Include a code snippet inline if it fits.
- Good: `"Add \`class Valves(BaseModel): ...\` inside the extension class."`
- Good: `"Set \`self.valves = self.Valves()\` in \`__init__\`."`
- Avoid vague advice like `"Fix the issue"` or `"See the docs"`.

**`help_url`** — Use an existing constant from the top of `src/rules.rs` (`PLUGIN_OVERVIEW`, `TOOLS_DOC`, `PIPE_DOC`, etc.) or add a new one. Point to the most specific Open WebUI docs page for the concept being checked.

## Rule Severity Model

- Every rule has a **default severity**: `error` or `warning`.
- Users can override severity in config with: `error`, `warning`, `off`.
- Exit behavior is controlled separately via CLI `--fail-on`:
  - `error`: fail only on errors
  - `warning`: fail on errors or warnings
  - `none`: never fail based on findings

## Add a New Rule (Checklist)

1. Add metadata in `src/rules.rs`.
2. Add lint logic in `src/linter.rs`.
3. Add tests in `tests/`.
4. Run quality checks.

Fast path (scaffold):

```bash
./scripts/new-rule.sh OWC600 warning "Missing cache timeout"
```

This adds a new constant + `RuleDoc` template to `src/rules.rs` and creates
`examples/rules/OWC600.md` with a contributor checklist.

Run `make test-scripts` to verify the scaffolding script works correctly.

### 1) Add rule metadata (`src/rules.rs`)

Add a new rule ID constant and a `RuleDoc` entry in `RULES`:

- `id` (`OW...`)
- `default_severity` (`Severity::Error` or `Severity::Warning`)
- `title`
- `summary`
- `remediation`
- `help_url`

The metadata powers:

- `owui-lint rules`
- `owui-lint explain <RULE_ID>`
- enriched text/json/github/sarif output

### 2) Emit findings from the linter (`src/linter.rs`)

Use the shared helper:

```rust
use crate::rules::{issue, MY_NEW_RULE_ID};

issues.push(issue(
    MY_NEW_RULE_ID,
    path,
    line,
    column,
    "Actionable message for users.",
));
```

This keeps severity aligned with `src/rules.rs` and avoids duplicated rule/severity definitions.

### 3) Add tests

At minimum:

- Add a case in `tests/linter_tests.rs` (or `tests/cli_tests.rs`) that triggers your new rule.
- Assert `rule_id` and expected severity.
- If needed, add config-override coverage (`error`/`warning`/`off`).

### 4) Run checks

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Docker Workflow (No Local Rust Required)

If you don't want to install Rust/Cargo locally:

```bash
make docker-build
make docker-run TARGET=.
make docker-install INSTALL_DIR="$HOME/.local/bin"
```

## Config Example (warning + error overrides)

```yaml
rules:
  OWT101: error   # promote default warning to error
  OWP202: warning # keep warning
  OWUI020: off    # disable rule
```
