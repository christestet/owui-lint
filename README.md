# owui-lint

![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust&logoColor=white)
![license](https://img.shields.io/badge/license-MIT-blue.svg)

> **Disclaimer:** `owui-lint` is a hobby project and is **not** developed or maintained by the
> [Open WebUI](https://github.com/open-webui/open-webui) team. Thank you to the Open WebUI
> maintainers for building an amazing platform. The rulesets in this tool are opinionated and
> derived from studying the Open WebUI extension codebase — they are **not** official rules
> endorsed by the Open WebUI project.

`owui-lint` is a Rust CLI linter for Open WebUI extensions:

- `Tools`
- `Pipe`
- `Filter`
- `Action`
- `Pipeline`

## Architecture

- Native binary distribution (`owui-lint`)
- YAML config for lint rules (`config.yml` or `owui-lint.yml`)

```mermaid
flowchart LR
    A["CLI (Rust / clap)"] --> B["File Discovery"]
    B --> C["Rust Analyzer"]
    C --> D["Rule Engine"]
    D --> E["Output: text/json/github/sarif"]
```

## Install

### Pre-built binaries (recommended)

macOS / Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/christestet/owui-lint/releases/latest/download/owui-lint-installer.sh | sh
```

Windows (PowerShell):

```powershell
irm https://github.com/christestet/owui-lint/releases/latest/download/owui-lint-installer.ps1 | iex
```

Or download a binary directly from the [Releases](https://github.com/christestet/owui-lint/releases/latest) page.

### Build from source

```bash
cargo build --release
./target/release/owui-lint
```

## Docker (No Rust/Cargo Needed)

Build image:

```bash
make docker-build
```

Run linter from Docker against current workspace:

```bash
make docker-run TARGET=.
make docker-run TARGET="path/to/extensions --format json --output report.json"
```

Install binary from Docker image to local `./bin`:

```bash
make docker-install
./bin/owui-lint --help
```

Install to a custom path:

```bash
make docker-install INSTALL_DIR="$HOME/.local/bin"
```

## Usage

Lint a single file:

```bash
owui-lint path/to/my/pipe.py
```

Lint a folder:

```bash
owui-lint path/to/my/extensions
```

Explicit lint subcommand (equivalent to positional mode):

```bash
owui-lint lint path/to/my/extensions
```

Choose output format:

```bash
owui-lint path/to/extensions --format text
owui-lint path/to/extensions --format github
owui-lint path/to/extensions --format json --output lint-report.json
owui-lint path/to/extensions --format sarif --output owui-lint.sarif
```

Discover and explain rules:

```bash
owui-lint rules
owui-lint rules --format json --output owui-rules.json
owui-lint explain OWT101
```

Update to the latest version:

```bash
owui-lint update
```

Control exit behavior:

```bash
owui-lint path/to/extensions --fail-on error
owui-lint path/to/extensions --fail-on warning
owui-lint path/to/extensions --fail-on none
```

Exit codes:

- `0`: no configured failure condition met
- `1`: failure condition met (`--fail-on`)
- `2`: usage/configuration/runtime error

Text output includes remediation hints per finding:

```text
path/to/tools.py:10:5: warning OWT101 Tool method 'search' should include a descriptive docstring.
  help: Tool methods should include clear docstrings so users understand capabilities.
  fix: Add a descriptive docstring to each public tool method.
```

## Configuration (`config.yml` or `owui-lint.yml`)

```yaml
lint:
  include:
    - "**/*.py"
  exclude:
    - ".git/**"
    - ".venv/**"
    - "**/__pycache__/**"

rules:
  # values: error | warning | off
  OWUI020: off
  OWT101: error
```

## Rules

Run `owui-lint rules` for the live catalog with current defaults.
Run `owui-lint explain <RULE_ID>` for per-rule details and remediation advice.

### Universal (all extension types)

| Rule | Severity | Title | What it checks |
|------|----------|-------|----------------|
| `OWUI001` | error | Python syntax error | File cannot be parsed as valid Python |
| `OWUI010` | warning | No extension class detected | File looks like an extension but has no `Tools`/`Pipe`/`Filter`/`Action`/`Pipeline` class |
| `OWUI011` | error | Mixed extension types | More than one extension class in a single file |
| `OWUI020` | warning | Missing Valves class | No inner `Valves` class for runtime configuration |
| `OWUI021` | warning | Valves should inherit BaseModel | `Valves` class does not extend `pydantic.BaseModel` |
| `OWUI022` | warning | Valves not initialized | `self.valves` not assigned in `__init__` |
| `OWUI030` | warning | Missing version in module header | Module docstring has no `version:` field |
| `OWUI031` | warning | Unpinned requirements in module header | `requirements:` entries lack version specifiers (`==`) |

### Tools (`OWT`)

| Rule | Severity | Title | What it checks |
|------|----------|-------|----------------|
| `OWT100` | error | No public tool methods | `Tools` class exposes zero callable public methods |
| `OWT101` | warning | Tool method missing docstring | A public tool method has no docstring |

### Pipe (`OWP`)

| Rule | Severity | Title | What it checks |
|------|----------|-------|----------------|
| `OWP200` | error | Pipe method missing | `Pipe` class has no `pipe` method |
| `OWP201` | warning | Pipe has inlet/outlet | `Pipe` defines filter-only hooks (`inlet`/`outlet`) |
| `OWP202` | warning | Pipe method should be async | `pipe` is defined as a synchronous function |

### Filter (`OWF`)

| Rule | Severity | Title | What it checks |
|------|----------|-------|----------------|
| `OWF300` | error | Filter has no inlet/outlet/stream | `Filter` implements none of the required hooks |
| `OWF301` | warning | inlet should return body | `Filter.inlet` has no `return body` statement |
| `OWF302` | warning | outlet should return body | `Filter.outlet` has no `return body` statement |

### Action (`OWA`)

| Rule | Severity | Title | What it checks |
|------|----------|-------|----------------|
| `OWA400` | error | Action method missing | `Action` class has no `action` method |
| `OWA401` | warning | Action should be async | `action` is defined as a synchronous function |

### Pipeline (`OWPL`)

| Rule | Severity | Title | What it checks |
|------|----------|-------|----------------|
| `OWPL500` | error | Pipeline missing processing hook | `Pipeline` defines neither `pipe` nor any filter hook |
| `OWPL501` | warning | Pipeline name not assigned | `self.name` not set in `Pipeline.__init__` |

## Rule Severity and Exit Behavior

`owui-lint` separates rule severity from CLI exit behavior:

- Rule severity: `error` or `warning` (default per rule, configurable)
- Exit policy: controlled by `--fail-on` (`none`, `error`, `warning`)

Example:

```yaml
rules:
  OWT101: error # turn warning into error
  OWP202: warning # keep warning
  OWUI020: off # disable rule
```

If a config contains unknown rule IDs, `owui-lint` warns and shows valid discovery commands.

## Contributing Rules

To add a new warning/error rule, see [CONTRIBUTING.md](CONTRIBUTING.md).

High-level flow:

1. Add rule metadata in `src/rules.rs` (including default severity and remediation).
2. Emit findings in `src/linter.rs` using the shared `issue(...)` helper.
3. Add tests in `tests/`.
4. Run `cargo fmt -- --check`, `cargo clippy -- -D warnings`, and `cargo test`.

Rule scaffold helper:

```bash
./scripts/new-rule.sh OWC600 warning "Missing cache timeout"
```

## SARIF for GitHub Code Scanning

```yaml
- name: Run owui-lint (SARIF)
  run: owui-lint path/to/extensions --format sarif --output owui-lint.sarif

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: owui-lint.sarif
```

## Quality

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## License

Check [MIT License](LICENSE)
