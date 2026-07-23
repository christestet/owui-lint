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
| `__init__` runs at load because OWUI **instantiates** the class | `plugin.py`: `return module.Tools()` / `module.Pipe()` / `module.Filter()` / `module.Action()` / `module.Event()` right after `exec` | `__init__` body (and `self.Valves()` it calls) runs on import — but only *because* OWUI constructs the object | always (verified) |
| `requirements` frontmatter → `pip install` via `subprocess.check_call` | `plugin.py` `install_frontmatter_requirements` | Supply chain (unpinned versions) | **only if `ENABLE_PIP_INSTALL_FRONTMATTER_REQUIREMENTS` and not `OFFLINE_MODE`** |
| `execute` event: unsandboxed JS via `new Function()` (DOM/cookies/localStorage) | Docs `events.mdx` → "execute" | Client-side exfiltration | depends on frontend/version |
| No execution timeout, tasks keep running after tab close | Docs `events.mdx` → "Persistence & Browser Disconnection" | Resource DoS / cryptomining | always |
| **`Event` functions auto-dispatch on system activity** (170+ events, `<area>.<action>`), not user action | `events.py` `dispatch_event_functions()` calls the instance's `event()` handler for every system event; `__app__` is injected, allowing route registration / app-state mutation | Broadens the import-time threat model into an **always-on, auto-triggered** attack surface: full server privileges with no user action required, admin-only creation is the only gate | always (when an `Event` class is present and defines `event()`) |
| Official threat list: exfiltrate data, malware, cryptomining, lateral movement | Docs `plugin-overview.mdx` "CRITICAL SECURITY WARNING" | — | — |

> **Why "cite by symbol, not line":** the bundled reference copy of `plugin.py`
> already drifted from upstream `main` (different line numbers for the same
> `exec`/instantiation). Line numbers rot; symbols don't. Pin claims to identifiers.

> **Two distinct loaders govern import-time execution — both are pulled as ground
> truth and the scope-aware pass must handle both:**
> | Loader (source) | Mechanism | Entry classes instantiated at load |
> |---|---|---|
> | `plugin.py` (`open-webui`) | `exec(content, module.__dict__)` then `module.Tools()` | `Tools`, `Pipe`, `Filter`, `Action`, `Event` |
> | `main.py` (`open-webui/pipelines`, **separate repo**) | `spec.loader.exec_module(module)` then `module.Pipeline()` | `Pipeline` |
>
> So `plugin.py` does **not** govern pipelines. Both run the module body at import and
> then construct the entry object (→ `__init__` is import-time in both). For
> `InitBody` scope detection, recognize **all six** class conventions
> (`Tools`/`Pipe`/`Filter`/`Action`/`Event`/`Pipeline`), not just `Tools`. Function valves are
> additionally re-instantiated at runtime by `utils/filter.py`
> (`function_module.Valves(**…)`) — that doesn't change import-time severity, but it's
> why `filter.py` is pulled too. `Event` functions are additionally re-instantiated
> per **dispatch** (`events.py` `dispatch_event_functions`, not just at import) and,
> unlike the other four, run **automatically** on system events rather than in
> response to a user action — see the new row in §1.1.

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

1. Add a scope tracker in `analysis/` (`ModuleLevel` / `InitBody` / `MethodBody` —
   `ValvesConstruct` was dropped, see §2). Unit tests with small snippets.
2. Implement **one** rule as proof: **`OWSEC001` — "Code execution at import time"**
   (trigger: `subprocess.*`, `os.system`, `eval`, `exec`, `__import__`, network calls
   at `ModuleLevel`/`InitBody`).
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

---

### Phase 1 — Implementation record (DONE ✅, 2026-06-09)

**Abort criterion passed → Phase 2 unblocked.** Built and committed-ready behind the
opt-in security profile, off by default.

**What shipped (where to look):**

- **Scope-aware call-site tracker.** `CallScope::{ModuleLevel,InitBody,MethodBody}` +
  `CallSite` in `src/models.rs`; `extract_calls` (string/comment-aware textual call
  detection) in `src/analysis/parsing.rs`; per-statement scope tagging in
  `src/analysis/mod.rs` (`current_call_scope`). `ModuleInfo` now carries
  `call_sites: Vec<CallSite>`.
- **`OWSEC001` — code execution at import time** (Error). `src/rules.rs` const + RuleDoc;
  detection `lint_security` + sink classifier `import_time_exec_category` in
  `src/linter.rs`. Sinks: `subprocess.*`, `os.system`/`os.popen`/`os.spawn*`, bare
  `eval`/`exec`/`__import__`/`compile`, network roots `requests`/`httpx`/`aiohttp`/
  `socket`/`urllib` — fired **only** at import scope.
- **Opt-in activation.** `Config.security` (default `false`) parsed from top-level
  `security: true`; `--security` CLI flag (OR-ed in, can only enable). Gated in
  `lint_module`. When off, OWSEC emits nothing.
- **Corpora.** Malicious fixtures `tests/fixtures/owsec/` (3 positives + 1 negative);
  integration tests `tests/owsec_tests.rs`; unit tests in `analysis/{mod,parsing}.rs`.
- **Docs/CLI.** OWSEC got its own generated rule group ("Security (`OWSEC`)") in
  `docs-sync`; README + `configuration.md` + `usage.md` document the profile.

**Measurement (the actual abort gate):**

- **Clean corpus** (`examples/**`, 33 files): **0 OWSEC001 findings** (threshold ≤2).
  Non-vacuous: the corpus has many `requests.`/`httpx.` calls (`jira_agent.py` ×7,
  `n8n_chats.py` ×8) all correctly classified `MethodBody` and suppressed; a sweep for
  indent-0 / bare-builtin sinks found none → no masked false negatives.
- **Malicious corpus**: **3/3** detected — `module_level_subprocess.py:12` (ModuleLevel),
  `init_network.py:20` (InitBody), `valves_field_default_eval.py:15` (class-body field
  default → ModuleLevel). Negative `clean_method_only.py` → 0.
- Gates green: `make check` (fmt/clippy/test/test-scripts/assets), `docs-sync --check`,
  `cargo audit` (0 vulns), `cargo machete` (0 unused), `cpd` (0.26% dup, none in new
  code). 156 tests pass.

**Decisions made (and why):**

1. **Engine: kept the structure-only indentation scanner; did NOT adopt
   `rustpython-parser`.** The PoC's job was to measure, and 0 FP / 3 TP says the simple
   engine is sufficient *for OWSEC001*. This stays a data-driven decision — see "Next".
2. **`InitBody` is restricted to the five entry classes** (`Tools`/`Pipe`/`Filter`/
   `Action`/`Pipeline`). Only these are instantiated at import by OWUI, so a helper
   class's `__init__` is *not* import-time and is classified `MethodBody`. This is the
   precise, verifiable reason `__init__` is import-time (`module.Tools()` right after
   `exec`, confirmed in synced `plugin.py:239/283/285/287`).
3. **Activation model pinned** (previously "left undecided"): a single boolean profile
   (`security`), not per-rule opt-in. CLI `--security` can only turn it *on*; it never
   overrides a config that enabled it. Rationale: one switch is the lowest-cognitive-load
   way to guarantee "no FP regression for existing users."
4. **The tracker records *all* call sites, not just sinks.** Keeps `analysis/` decoupled
   from rule-specific sink lists so later OWSEC rules (010/011/020/021/030) reuse it; the
   per-file cost is negligible.
5. **Call detection is callee-name + scope only** — deliberately no argument inspection,
   matching the §2.1 scope-honesty stance. `self.exec(...)` reads as `self.exec` (not the
   bare builtin `exec`); a space before `(` breaks the match (accepted heuristic).

**Assumptions & known limitations (carry into Phase 2):**

- **`if __name__ == "__main__":` guards** are currently treated as `ModuleLevel` even
  though that block does **not** run when OWUI imports the file as a module. Potential
  FP source; not observed in the corpus. Add a guard if Phase 2 surfaces it.
- **Multi-line calls** are detected on the line bearing the `(`; deeply-indented
  continuation lines are line-scanned independently (pre-existing scanner trait). Fine
  for sink detection, which only needs the callee line.
- **A line opening a triple-quoted string** (`x = """…`) stops call extraction at the
  quote — a sink call *before* the opening quote on that same line would be missed
  (rare).
- **Decorator/default-arg calls on `def`/`class` header lines** are not scanned (those
  lines `continue` before call extraction). Class-body field defaults *are* scanned.
- **0 FP on 33 curated, well-formed files is a low-confidence denominator.** Real-world
  messy plugins are the true FP distribution — see "Next".

**Deviations from the original plan:**

- Dropped `ValvesConstruct` scope (already foreseen in §2) — field defaults map to
  `ModuleLevel`, hand-written `Valves.__init__` to `InitBody`/`MethodBody`.
- Added a small `error_module()` constructor in `analysis/mod.rs` (DRY: cpd flagged the
  duplicated empty-`ModuleInfo` literals after the `call_sites` field was added).

**What's next (recommended order before/within Phase 2):**

1. **Commit Phase 1** behind the flag (isolated, tested, off-by-default). Mark OWSEC
   "preview."
2. **Timeboxed `rustpython-parser` spike** — port OWSEC001 to a real parse tree, rerun
   the *same* corpora, compare FP/FN + code complexity. Decide the engine **with numbers
   before** writing rules 2–9, when migration is cheapest. (Phase 1 says "only if FP
   forces it"; the senior-review refinement is: prove it now rather than discover it at
   rule 9.)
3. **Severity model** — the `Severity` enum is Error/Warning only (the `new-rule.sh`
   script test literally *rejects* `info`). OWSEC031/050 want `Info`. Design the
   taxonomy **once**: ideally separate axes — `level` (error/warning/info) ×
   `confidence` (security findings are probabilistic, cf. Bandit) × `default_enabled` —
   rather than overloading a single `Info` level. `[OPEN DECISION]`
4. **Test/corpus infrastructure** — grow the malicious corpus, add a held-out real-world
   corpus (scrape community Tools/Functions), and add snapshot testing so each new rule's
   output is pinned and diffable. This is what makes rules 2–9 cheap and safe.
5. **Inline suppression** (`# owui-lint: ignore[OWSEC001]`) + optional baseline file —
   an adoption gate for security rules before they go wide. `[OPEN DECISION]`
6. *Then* Phase 2 rule catalog (below), which becomes largely declarative.

---

### Phase 2 — `OWSEC` rule catalog

Implement in `rules.rs` + `analysis/`, each with tests in `tests/rules_tests.rs`.
Severity proposal (finalize while building):

Each trigger below is **scopeable** (presence + scope) unless marked. Rows are
rescoped from the original draft so we promise only what the structure-only engine can
prove (see §2.1); the dropped data-flow framing is noted inline.

| ID | Title | Trigger (what we actually detect) | Default severity | Scope-dependent? |
|---|---|---|---|---|
| `OWSEC001` ✅ **shipped (Phase 1)** | Code execution at import time | `subprocess`/`os.system`/`eval`/`exec`/`__import__`/network call at `ModuleLevel`/`InitBody` | Error | yes (import scopes only) |
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

> **FP discipline & activation (DECIDED in Phase 1):** All `OWSEC` rules default to
> non-interference via a single opt-in profile — `Config.security` (top-level
> `security: true`) or the `--security` CLI flag (enable-only). When the profile is off,
> no `OWSEC` rule emits anything; when on, severities are still tunable via `rules:`
> overrides. The hard requirement — no FP regression for existing users — is met because
> the gate sits in `lint_module` before any OWSEC detection runs.

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

> **Phase 1 is DONE** (see §3 "Phase 1 — Implementation record"): the scope-aware
> tracker + `OWSEC001` shipped behind the opt-in `--security` profile, 0 FP on
> `examples/**`, 3/3 fixtures detected.
>
> "Read `OWUI_SEC.md` (esp. the Phase 1 record + 'What's next') and
> `docs/adr/0001-security-trust-positioning.md`. Before extending the catalog, run the
> **timeboxed `rustpython-parser` spike** (port `OWSEC001`, rerun the corpora, compare
> FP/FN + complexity) and settle the **severity taxonomy** (`level × confidence ×
> default_enabled`) since `Info` doesn't exist yet. Then build the **Phase 2** rules
> (`OWSEC010`–`OWSEC050`) on whichever engine the spike justifies. Verify with
> `make docker-check`."
