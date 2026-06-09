# OWUI_SEC — Work Plan: Security & Trust Rule Class for owui-lint

> **Purpose of this document:** A self-contained implementation plan so a fresh
> session can start without prior context. Read this plan in full first, then
> [ADR 0001](docs/adr/0001-security-trust-positioning.md) for the strategic "why".

---

## 0. TL;DR

We are building a new rule class **`OWSEC`** (security) plus a **trust-report output
format**. Core idea: Open WebUI executes plugin code via `exec()` **at import time**
with full server privileges. A security finding is significantly more severe when it
sits at **module/`__init__` level** (runs immediately on import) versus in a method
body (runs only when called). This scope distinction is our differentiator versus
Bandit/Ruff.

**Order:** scope-aware AST pass first (foundation) → then the first `OWSEC` rule as a
PoC → then the rest of the rule catalog → then the trust-report format.

---

## 1. Mandatory context (read before coding)

### 1.1 Threat model (backed by OWUI primary sources)

| Vector | Evidence | Severity driver |
|---|---|---|
| Arbitrary Python via `exec(content, module.__dict__)` at **import time** | `backend/open_webui/utils/plugin.py`, local: `.agents/openwebui-extensions/references/plugin.py` (~L199–290) | Code at module/`__init__` level runs without user consent |
| `requirements` frontmatter → `pip install` via `subprocess.check_call` | Docs "External packages"; `plugin.py` `install_frontmatter_requirements` | Supply chain / typosquatting |
| `execute` event: unsandboxed JS via `new Function()` (DOM/cookies/localStorage) | Docs "events" → "execute" | Client-side exfiltration |
| No execution timeout, tasks keep running after tab close | Docs "events" → "Persistence & Browser Disconnection" | Resource DoS / cryptomining |
| Official threat list: exfiltrate data, malware, cryptomining, lateral movement | Docs plugin overview "CRITICAL SECURITY WARNING" | — |

**Source URLs (verified, resolve correctly):**
- Plugin overview + security warning: <https://docs.openwebui.com/features/extensibility/plugin/>
- Tool development + External packages: <https://docs.openwebui.com/features/extensibility/plugin/tools/development>
- Events (`execute`, timeout): <https://docs.openwebui.com/features/extensibility/plugin/development/events>
- Valves (secrets/password input): <https://docs.openwebui.com/features/extensibility/plugin/development/valves>
- Backend `plugin.py`: <https://raw.githubusercontent.com/open-webui/open-webui/refs/heads/main/backend/open_webui/utils/plugin.py>

> ⚠️ **URL gotcha:** The docs live under `/features/extensibility/plugin/...`,
> **not** under `/features/plugin/...` (the latter → 404).

### 1.2 Existing architecture (do not reinvent)

- `src/analysis/` — AST/Python scan. `mod.rs` (orchestration), `parsing.rs`
  (Python construct parsing), `syntax.rs` (delimiter/string detection, `OWUI001`).
- `src/rules.rs` — rule catalog. `RuleDoc` struct with `id`, `default_severity`,
  `title`, `summary`, `remediation`, `help_url`, `openwebui_version`. Rule IDs as
  `pub const` (prefixes: `OWUI`, `OWT`, `OWP`, `OWF`, `OWA`, `OWPL`).
- `src/linter.rs` — orchestration scan → rules → issues.
- `src/models.rs` — `Issue`, `Severity`.
- `src/output.rs` — reporters: text / json / github / sarif.
- `src/config.rs` — YAML config (`owui-lint.yml`).
- Tests: `tests/` (integration: `rules_tests.rs`, `linter_tests.rs`,
  `multiline_test.rs`), inline `#[cfg(test)]` (unit).

> **Important:** The current scanner is **not** a full Python parser (deliberately,
> see README "Scope and Limits"). For `OWSEC` we need more scope information, but
> **not** full grammar parsing. See Phase 1.

### 1.3 Verification (mandatory before every commit)

```
make docker-check   # all quality gates, CI parity
```

New-rule helpers exist: `scripts/new-rule.sh`, `scripts/test-new-rule.sh`.
Docs sync: `src/bin/docs-sync.rs` (rules ↔ README/docs). Check docs sync before commit.

---

## 2. Architecture decision: scope-aware AST pass

**Problem:** Security rules with a high false-positive rate are toxic. We must
distinguish:
- Does a `subprocess.run(...)` site run at **module level** / in `__init__` / in a
  `Valves` constructor (→ import time, high severity) or in a **method body**
  (→ lower severity)?
- Is `eval` a **call** or just a string/comment?

**Decision (to be validated in the spike):** A minimally invasive scope tracker in
`analysis/` that carries a **scope context** per detected "call/attribute access":

- `ModuleLevel` — top-level statements of the file
- `InitBody` — inside `def __init__`
- `ValvesConstruct` — inside `class Valves`/`UserValves` body / field defaults
- `MethodBody` — inside any other method

No full AST required: indentation/block tracking on top of the existing `parsing.rs`
(class/function detection already exists for the structural rules) suffices as a first
step. **Alternative to evaluate:** a real lightweight AST via `rustpython-parser` —
only if the indentation-based approach produces too many FPs. This trade-off is part
of the PoC (Phase 3), not to be decided up front.

---

## 3. Phase plan

### Phase 1 — Spike / PoC (goal: feasibility + measure FP rate)

1. Add a scope tracker in `analysis/` (`ModuleLevel` / `InitBody` /
   `ValvesConstruct` / `MethodBody`). Unit tests with small snippets.
2. Implement **one** rule as proof: **`OWSEC001` — "Code execution at import time"**
   (trigger: `subprocess.*`, `os.system`, `eval`, `exec`, `__import__`, network calls
   at `ModuleLevel`/`InitBody`/`ValvesConstruct`).
3. Run against the corpus: `examples/**` (real community plugins in the repo) →
   manually review FP/TP rate. **Abort criterion:** if the FP rate on `examples/` is
   untenable, switch approaches (rustpython-parser) before building further.

**Definition of Done Phase 1:** `OWSEC001` runs, `make docker-check` green, FP review
on `examples/` documented.

### Phase 2 — `OWSEC` rule catalog

Implement in `rules.rs` + `analysis/`, each with tests in `tests/rules_tests.rs`.
Severity proposal (finalize while building):

| ID | Title | Trigger | Default severity | Scope-dependent? |
|---|---|---|---|---|
| `OWSEC001` | Code execution at import time | `subprocess`/`os.system`/`eval`/`exec`/`__import__`/network at module/init level | Error | yes (import scopes only) |
| `OWSEC010` | Subprocess / shell execution | `subprocess.*`, `os.system`, `os.popen`, `pty.spawn` | Warning (Error in import scope) | severity-raising |
| `OWSEC011` | Dynamic code evaluation | `eval`, `exec`, `compile`, `__import__` with dynamic arg | Warning (Error in import scope) | severity-raising |
| `OWSEC020` | Outbound network at import time | `requests`/`httpx`/`aiohttp`/`socket`/`urllib` at module/init level | Error | yes |
| `OWSEC021` | Filesystem write outside data dir | `open(..., 'w'/'a')`, `os.remove`, `shutil.*`, `pathlib.write_*` | Warning | optional |
| `OWSEC030` | Environment / secret access | reading `os.environ` + forwarding to an external call (exfiltration pattern) | Warning | yes |
| `OWSEC031` | Hardcoded secret / token | plaintext token patterns (`sk-`, `AKIA`, long hex/base64) in source | Warning | no |
| `OWSEC040` | Risky `requirements` frontmatter | unpinned / suspicious PyPI names in frontmatter `requirements:` | Warning | no |
| `OWSEC050` | Unsandboxed `execute` JS event | `__event_emitter__`/`__event_call__` with `type: "execute"` + dynamic `code` | Info/Warning | no |

> **FP discipline:** All `OWSEC` rules active only under an opt-in `--security` profile
> OR with conservative defaults that do not interfere in the standard run. Decide the
> exact activation model while building (config flag in `config.rs`).

**Per-rule checklist:**
- [ ] `pub const OWSECxxx` + `RuleDoc` entry (with `help_url`, `openwebui_version`)
- [ ] Detection in `analysis/` + wiring in `linter.rs`
- [ ] Tests: positive case, negative case, scope variation (import vs. method)
- [ ] Docs sync (`docs-sync`) + README updated
- [ ] `make docker-check` green

### Phase 3 — Trust-report output format

A new format in `output.rs` (e.g. `--format trust`) that condenses `OWSEC` findings
per file into an **admin-readable risk verdict**. Sketch:

```
Plugin: weather_tool.py
  Network access ........ yes (import-time!)   ← OWSEC020
  Filesystem writes ..... no
  Subprocess / shell .... no
  Dynamic eval/exec ..... no
  Env / secrets ......... reads os.environ     ← OWSEC030
  requirements (pip) .... 3 packages, 1 unpinned ← OWSEC040
  Import-time execution . YES                  ← OWSEC001
  ─────────────────────────────────────────────
  RISK: HIGH   (1 critical, 1 medium)
```

- Aggregation: map `OWSEC` IDs → trust dimensions + risk-score function.
- A machine-readable variant (JSON) in parallel, for CI gates.
- DoD: `owui-lint <dir> --format trust` produces the above; snapshot test.

### Phase 4 — CI distribution (separate, after Phase 3)

- GitHub Action (`action.yml`) running `--security` + the trust gate.
- pre-commit hook definition.
- SARIF remains for code scanning; the trust report is for humans.
- (Not part of this plan in detail — a separate follow-up task.)

---

## 4. Explicitly NOT in scope (deliberately deferred)

- **Python binding / maturin API** — only with a concrete adapter (see ADR 0001 §6).
- **pip/pipx distribution** — reach multiplier, after the security axis.
- **Version/compatibility mode** — only with a contributor commitment (maintenance burden).
- **Full Python parser** — only if the Phase 1 FP rate forces it.

---

## 5. Kickoff prompt for the new session

> "Read `OWUI_SEC.md` and `docs/adr/0001-security-trust-positioning.md`. Start with
> **Phase 1**: scope-aware AST pass + `OWSEC001` as a PoC. Measure the FP rate against
> `examples/**` and stop before Phase 2 if the abort criterion is hit. Verify with
> `make docker-check`."
