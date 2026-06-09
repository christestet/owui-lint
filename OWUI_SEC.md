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

| Vector | Evidence (cite by symbol, not line) | Severity driver | Conditional? |
|---|---|---|---|
| Module body runs at load via `exec(content, module.__dict__)` | `plugin.py` `load_tool_module_by_id` / `load_function_module_by_id` | Top-level + class-body code (incl. Pydantic field defaults) runs on import, no user consent | always |
| `__init__` runs at load because OWUI **instantiates** the class | `plugin.py`: `return module.Tools()` / `module.Pipe()` / `module.Filter()` / `module.Action()` right after `exec` | `__init__` body (and `self.Valves()` it calls) runs on import — but only *because* OWUI constructs the object | always (verified) |
| `requirements` frontmatter → `pip install` via `subprocess.check_call` | `plugin.py` `install_frontmatter_requirements` | Supply chain (unpinned versions) | **only if `ENABLE_PIP_INSTALL_FRONTMATTER_REQUIREMENTS` and not `OFFLINE_MODE`** |
| `execute` event: unsandboxed JS via `new Function()` (DOM/cookies/localStorage) | Docs `events.mdx` → "execute" | Client-side exfiltration | depends on frontend/version |
| No execution timeout, tasks keep running after tab close | Docs `events.mdx` → "Persistence & Browser Disconnection" | Resource DoS / cryptomining | always |
| Official threat list: exfiltrate data, malware, cryptomining, lateral movement | Docs `plugin-overview.mdx` "CRITICAL SECURITY WARNING" | — | — |

> **Why "cite by symbol, not line":** the bundled reference copy of `plugin.py`
> already drifted from upstream `main` (different line numbers for the same
> `exec`/instantiation). Line numbers rot; symbols don't. Pin claims to identifiers.

> **Two distinct loaders govern import-time execution — both are pulled as ground
> truth and the scope-aware pass must handle both:**
> | Loader (source) | Mechanism | Entry classes instantiated at load |
> |---|---|---|
> | `plugin.py` (`open-webui`) | `exec(content, module.__dict__)` then `module.Tools()` | `Tools`, `Pipe`, `Filter`, `Action` |
> | `main.py` (`open-webui/pipelines`, **separate repo**) | `spec.loader.exec_module(module)` then `module.Pipeline()` | `Pipeline` |
>
> So `plugin.py` does **not** govern pipelines. Both run the module body at import and
> then construct the entry object (→ `__init__` is import-time in both). For
> `InitBody` scope detection, recognize **all five** class conventions
> (`Tools`/`Pipe`/`Filter`/`Action`/`Pipeline`), not just `Tools`. Function valves are
> additionally re-instantiated at runtime by `utils/filter.py`
> (`function_module.Valves(**…)`) — that doesn't change import-time severity, but it's
> why `filter.py` is pulled too.

**Ground truth (don't assume — fetch and diff):** the upstream files every OWSEC
rule is designed against are pulled by `scripts/sync-owui-sources.sh` into
`.agents/openwebui-extensions/references/` with provenance in `SOURCES.md`
(URL + fetch date + sha256). Run `make owui-sources-check` to alarm on upstream
drift before trusting a citation. Sources (all verified HTTP 200):
- Plugin overview + security warning: <https://docs.openwebui.com/features/extensibility/plugin/>
- Tool development + External packages: <https://docs.openwebui.com/features/extensibility/plugin/tools/development>
- Events (`execute`, timeout): <https://docs.openwebui.com/features/extensibility/plugin/development/events>
- Valves (secrets/password input): <https://docs.openwebui.com/features/extensibility/plugin/development/valves>
- Backend `plugin.py`: <https://raw.githubusercontent.com/open-webui/open-webui/refs/heads/main/backend/open_webui/utils/plugin.py>

> ⚠️ **URL gotcha:** The docs live under `/features/extensibility/plugin/...`,
> **not** under `/features/plugin/...` (the latter → 404). The docs *repo* paths
> (for the sync script) are `open-webui/docs` → `docs/features/extensibility/plugin/...`.

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

**Upstream drift check (OWSEC-specific):** `make owui-sources-check` re-fetches the
OWUI ground-truth files and fails if our bundled copies are stale. It is **not** part
of `make docker-check` (it needs network; offline/Docker builds must still pass).
Run it when touching OWSEC rules or on a schedule, and re-verify affected rules if it
reports drift.

---

## 2. Architecture decision: scope-aware AST pass

**Problem:** Security rules with a high false-positive rate are toxic. We must
distinguish:
- Does a `subprocess.run(...)` site run at **module level** / in `__init__` / in a
  `Valves` constructor (→ import time, high severity) or in a **method body**
  (→ lower severity)?
- Is `eval` a **call** or just a string/comment?

**Reuse, don't reinvent:** `analysis/mod.rs` already maintains a `contexts` stack
(`Context::Class` / `Context::Function`) with indent tracking, multiline-string
handling, and an `is_init_method` flag. The "scope tracker" is largely **already
there** — what's missing is recording **call sites** (currently the parser records
structure only: classes, methods, imports, valve fields; it does **not** record
expressions/calls). The new work is: detect call sites and tag each with the scope it
sits in.

**Scope contexts** carried per detected call/attribute access:

- `ModuleLevel` — top-level statements **and class-body / Pydantic field-default
  expressions**. Field defaults like `x: str = subprocess.run(...)` execute during
  `exec` (class definition time), i.e. import time — so they belong here, not in a
  separate "Valves" scope.
- `InitBody` — inside `def __init__`. Runs at import time **transitively**: only
  because OWUI calls `module.Tools()`/`module.Pipe()`/… right after `exec`. Same
  import-time severity as `ModuleLevel` in practice.
- `MethodBody` — any other method. Runs only when invoked → lower severity.

> **Dropped `ValvesConstruct` as a distinct scope.** It conflated two things with
> different runtime timing: Pydantic field-default *expressions* (→ `ModuleLevel`,
> run during `exec`) and a hand-written `def __init__` inside `Valves` (→ `InitBody`,
> runs only if the class is instantiated). Keeping it would have encoded a wrong
> mental model. Validate the field-default timing against synced `plugin.py` behavior.

**Engine reality (hard constraint):** the scanner is structure-only — **no variable
tracking, no expression evaluation, no data flow**. Any rule needing "value X reaches
sink Y" (taint) is **out of reliable scope** with this engine; see §2.1.

No full AST required: indentation/block tracking on top of the existing `parsing.rs`
suffices as a first step. **Alternative to evaluate:** a real lightweight AST via
`rustpython-parser` — only if the indentation-based approach produces too many FPs.
This trade-off is part of the PoC, not to be decided up front.

---

## 2.1 What the engine can and cannot decide (scope honesty)

To avoid promising detections we cannot deliver, every OWSEC rule is classified by
what the structure-only engine can actually prove:

**Scopeable (presence + scope):** "does call `X` appear, and in which scope?" — e.g.
`subprocess.run`, `eval`, `requests.get`, `open(..., 'w')`, `os.environ`. These are
textual call-site detections tagged with the scope stack. ✅

**Heuristic (textual, FP-prone — ship Info/off-by-default):** hardcoded-secret regexes
(`sk-`, `AKIA`, long hex/base64); "is the sole arg a string literal vs a name" (a
*weak* dynamic-vs-literal signal, not real arg analysis). ⚠️

**Out of reliable scope (needs data flow / taint — DO NOT promise):**
- "reads `os.environ` **and forwards it to an external call**" (exfiltration). We can
  only see the read and the call separately, not that one feeds the other.
- "filesystem write **outside the data dir**" — needs path resolution + `DATA_DIR`.
- "`eval` **with a dynamic argument**" / "`execute` event with **dynamic** `code`" —
  needs to prove the argument is attacker-controlled.

**Decision:** taint-dependent rules are **rescoped to their scopeable core** (presence
+ scope) for now, not half-built. The data-flow ambition (would require
`rustpython-parser` + a taint pass) is recorded in §4 as explicitly deferred. Whatever
we *do* ship is validated against the synced upstream sources (`make
owui-sources-check`), so the behavior we assume is the behavior OWUI implements.

## 3. Phase plan

### Phase 1 — Spike / PoC (goal: feasibility + measure FP rate)

1. Add a scope tracker in `analysis/` (`ModuleLevel` / `InitBody` /
   `ValvesConstruct` / `MethodBody`). Unit tests with small snippets.
2. Implement **one** rule as proof: **`OWSEC001` — "Code execution at import time"**
   (trigger: `subprocess.*`, `os.system`, `eval`, `exec`, `__import__`, network calls
   at `ModuleLevel`/`InitBody`/`ValvesConstruct`).
3. Build the two corpora the measurement needs (the repo has only one today):
   - **Clean corpus = `examples/**`** (32 real community plugins). These are
     well-formed; expect them to contain **~zero** real import-time-exec findings. So
     here every finding is a **candidate false positive** — the FP signal.
   - **Malicious corpus = new `tests/fixtures/owsec/`** — small hand-written plugins
     that genuinely execute at import time (the true positives). Without this, "TP
     rate" is undefined; there is nothing in `examples/` to detect.
4. Manually classify every finding. **Concrete abort criterion** (replaces "untenable"):
   - **> 2 false positives across all 32 `examples/` files**, **or**
   - **any malicious fixture missed** (false negative on an obvious import-time `exec`),
   → stop and switch to `rustpython-parser` before building Phase 2.

**Definition of Done Phase 1:** `OWSEC001` runs; malicious fixtures detected; FP count
on `examples/` ≤ 2 and documented (file + line for each); `make docker-check` green.

> **Phase 1 status: DONE (abort criterion passed).** The scope-aware call-site tracker
> (`CallScope::{ModuleLevel,InitBody,MethodBody}` in `models.rs`, `extract_calls` in
> `analysis/parsing.rs`, scope tagging in `analysis/mod.rs`) and `OWSEC001` ship behind
> an opt-in security profile (`--security` / `security: true`, off by default — no FP
> regression for existing users). Measurement:
> - **Clean corpus** (`examples/**`, 33 files): **0 OWSEC001 findings** — well under the
>   ≤2 threshold. Verified non-vacuous: the corpus contains many `requests.`/`httpx.`
>   sink calls (e.g. `jira_agent.py` ×7, `n8n_chats.py` ×8) all correctly classified as
>   `MethodBody` and suppressed; a sweep for indent-0 / bare-builtin sinks found none, so
>   no false negatives are masked.
> - **Malicious corpus** (`tests/fixtures/owsec/`): 3/3 detected — `module_level_subprocess.py:12`
>   (ModuleLevel), `init_network.py:20` (InitBody), `valves_field_default_eval.py:15`
>   (class-body field default → ModuleLevel). Negative fixture `clean_method_only.py` → 0.
>
> The indentation-based engine was sufficient; **`rustpython-parser` was not needed**.
> Phase 2 is unblocked. Note: code guarded by `if __name__ == "__main__":` is currently
> treated as `ModuleLevel` even though it does not run under `exec` as a module — a known
> potential FP source, not observed in the corpus; add a guard if Phase 2 surfaces it.

### Phase 2 — `OWSEC` rule catalog

Implement in `rules.rs` + `analysis/`, each with tests in `tests/rules_tests.rs`.
Severity proposal (finalize while building):

Each trigger below is **scopeable** (presence + scope) unless marked. Rows are
rescoped from the original draft so we promise only what the structure-only engine can
prove (see §2.1); the dropped data-flow framing is noted inline.

| ID | Title | Trigger (what we actually detect) | Default severity | Scope-dependent? |
|---|---|---|---|---|
| `OWSEC001` | Code execution at import time | `subprocess`/`os.system`/`eval`/`exec`/`__import__`/network call at `ModuleLevel`/`InitBody` | Error | yes (import scopes only) |
| `OWSEC010` | Subprocess / shell execution | `subprocess.*`, `os.system`, `os.popen`, `pty.spawn` (anywhere) | Warning (Error in import scope) | severity-raising |
| `OWSEC011` | Dynamic code evaluation | `eval`, `exec`, `compile`, `__import__` call present. *Optional weak suppression when the sole arg is a string literal — NOT a dynamic-arg proof* | Warning (Error in import scope) | severity-raising |
| `OWSEC020` | Outbound network at import time | `requests`/`httpx`/`aiohttp`/`socket`/`urllib` call at `ModuleLevel`/`InitBody` | Error | yes |
| `OWSEC021` | Filesystem write at import time | `open(..., 'w'/'a')`, `os.remove`, `shutil.*`, `pathlib.write_*` at import scope. *(Rescoped: was "outside data dir" — that needs path resolution, out of scope §2.1)* | Warning | yes |
| `OWSEC030` | Environment / secret read at import time | `os.environ` / `os.getenv` read at `ModuleLevel`/`InitBody`. *(Rescoped: was "+ forwarding to external call" — taint, out of scope §2.1)* | Info/Warning | yes |
| `OWSEC031` | Hardcoded secret / token | plaintext token patterns (`sk-`, `AKIA`, long hex/base64) in source. *Heuristic, FP-prone → Info, off by default* | Info | no |
| `OWSEC040` | Unpinned `requirements` frontmatter | requirement in frontmatter `requirements:` with no version pin (`==`). *(Rescoped: dropped "suspicious/typosquatting" — needs a package DB. Parse frontmatter OWUI's way: first line must be exactly `"""`. Finding is conditional on `ENABLE_PIP_INSTALL_FRONTMATTER_REQUIREMENTS`.)* | Info/Warning | no |
| `OWSEC050` | Unsandboxed `execute` JS event | `__event_emitter__`/`__event_call__` call with `type: "execute"`. *(Rescoped: dropped "+ dynamic code" — taint. Detect the event type only.)* | Info | no |

> **Frontmatter trap (OWSEC040):** OWUI's `extract_frontmatter` only treats a docstring
> as frontmatter when **`lines[0].strip() == '"""'`** — no prefix (`r"""`), no leading
> blanks/comments. owui-lint's existing `extract_module_docstring` is more permissive,
> so do **not** reuse it for OWSEC040; match OWUI's stricter rule or we flag/miss what
> OWUI itself ignores. Validate against synced `plugin.py`.

> **FP discipline & activation:** All `OWSEC` rules must default to non-interference in
> the standard run (opt-in `--security` profile and/or conservative defaults). **Exact
> activation model is left undecided** and will be pinned while building (config flag in
> `config.rs`); the only hard requirement is no FP regression for existing users.

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

- **Data-flow / taint analysis** — the rescoped-away parts of `OWSEC030` (env →
  external-call exfiltration), `OWSEC021` ("outside data dir"), `OWSEC011`/`OWSEC050`
  (proving an arg is dynamic/attacker-controlled). Requires `rustpython-parser` + a
  taint pass. Revisit only if the presence+scope versions prove insufficient in
  practice.
- **Typosquatting / suspicious-package detection** (`OWSEC040`) — needs a maintained
  package database; out of scope as pure-textual.
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
