# ADR 0001: Positioning as an Independent Security and Trust Layer for Open WebUI Extensions

- **Status:** Accepted
- **Date:** 2026-06-09
- **Affects:** Product strategy, roadmap, architecture (`analysis/`, `rules.rs`, `output.rs`)
- **Context version:** owui-lint 0.8.4

## Context

`owui-lint` today is a CLI linter whose ~26 rules almost exclusively check
**structural/semantic correctness** (extension-type structure, Valves,
reserved-args/signatures, async conventions). Security is currently covered by a
single rule (`OWUI022`, "Sensitive valve field not masked").

The strategic question was which direction of expansion delivers the greatest value
relative to the project's positioning against Open WebUI. Options discussed:

1. Expand security/trust rules
2. Python distribution via `maturin`/pip/pipx (not an API binding)
3. Strengthen the CI/community workflow (GitHub Action, pre-commit, SARIF-first, quality gate)
4. Version/compatibility mode per Open WebUI version
5. Sharpen the import-review story ("would I install this on my server?")
6. Python binding (in-memory linting of source strings)

## Evidence from Official Open WebUI Sources

This decision is not opinion-based; it is grounded in primary sources.

### 1. Open WebUI names the problem itself — prominently

The plugin overview page carries a **`⚠️ CRITICAL SECURITY WARNING`** at the very top:

> "Tools, Functions, Pipes, Filters, and Pipelines execute arbitrary Python code on
> your server. This is by design."

With five mandates for admins: *only install from trusted sources*, **review code
before importing**, *protect your data directory*, *restrict Workspace access*, and
**audit installed plugins regularly**. Plus the threat list: *exfiltrate data, install
malware, mine cryptocurrency, pivot to other systems on your network, corrupt your
instance*.

The tool development page sharpens this literally:

> "Granting a user the ability to create or import Tools is equivalent to giving them
> shell access to the server."

Sources:
- <https://docs.openwebui.com/features/extensibility/plugin/>
- <https://docs.openwebui.com/features/extensibility/plugin/tools/development>

### 2. The loading mechanism proves import-time execution technically

In the backend original (`backend/open_webui/utils/plugin.py`,
`load_function_module_by_id` / `load_tool_module_by_id`), two steps run back-to-back:

```python
exec(content, module.__dict__)        # 1. runs the entire file body
...
return module.Tools(), frontmatter    # 2. immediately INSTANTIATES the class
# (likewise module.Pipe() / module.Filter() / module.Action())
```

Two distinct import-time execution paths follow, and the distinction matters for how
we assign scope severity:

1. **Module body + class-body / Pydantic field defaults** run in step 1 (`exec`).
   `x: str = subprocess.run(...)` as a Valves field default executes here — at class
   *definition* time, i.e. import time.
2. **`__init__` bodies** run in step 2 — but **only because OWUI itself constructs the
   object** (`module.Tools()`). `exec` alone does *not* run `__init__`; the
   instantiation does. This is the precise, verifiable reason `__init__` (and any
   `self.Valves()` it calls) is import-time.

Either way: code at module/init scope runs **with no tool call and no user consent**,
making "network call at import time" or "`subprocess` in the class body" a
qualitatively sharper threat than the same code in a method body. This scope
distinction is **not** something generic linters (Bandit/Ruff) assess in the OWUI
context.

> **Evidence is pinned by symbol, not line number.** The bundled reference copy of
> `plugin.py` already drifted from upstream `main` (same code, different lines), so
> citing line ranges would encode a falsehood. `scripts/sync-owui-sources.sh` pulls the
> current upstream source/docs with provenance (`SOURCES.md`), and `make
> owui-sources-check` alarms on drift so claims are re-verified against the real
> implementation rather than a frozen snapshot.

Source: <https://raw.githubusercontent.com/open-webui/open-webui/refs/heads/main/backend/open_webui/utils/plugin.py>

### 3. `requirements` frontmatter → `pip install` (supply-chain vector)

```python
install_frontmatter_requirements(frontmatter.get("requirements", ""))
# → subprocess.check_call([sys.executable, "-m", "pip", "install", ...])
```

A plugin can install **arbitrary PyPI packages** via its frontmatter (when
`ENABLE_PIP_INSTALL_FRONTMATTER_REQUIREMENTS` is enabled). The docs themselves warn
about non-deterministic install order and version conflicts and recommend
`ENABLE_PIP_INSTALL_FRONTMATTER_REQUIREMENTS=False` for production. A classic
typosquatting/supply-chain vector that no one else checks in a plugin-specific way.

Source: <https://docs.openwebui.com/features/extensibility/plugin/tools/development> ("External packages" section)

### 4. `execute` event: unsandboxed JavaScript in the browser context

The `execute` event runs JS via `new Function()` **in the main page context** — full
access to DOM, cookies, `localStorage`, session; **not** sandboxed. The docs
themselves: "Only use this in trusted functions — never execute untrusted or
user-provided code through this event." A second, client-side
exfiltration/hijacking vector.

Source: <https://docs.openwebui.com/features/extensibility/plugin/development/events> ("execute" section)

### 5. No execution timeout

> "NO EXECUTION TIMEOUT — There is no timeout on pipe, tool, or action execution.
> Your code can run for minutes or hours."

Plus: server-side `asyncio` tasks keep running after the tab is closed. This supports
the "mine cryptocurrency" / resource-DoS threat from the official warning.

Source: <https://docs.openwebui.com/features/extensibility/plugin/development/events> ("Persistence & Browser Disconnection" section)

## Decision

`owui-lint` positions itself as an **independent security and trust layer for the
Open WebUI plugin ecosystem** — not primarily as a correctness linter, and explicitly
**not** as an "official policy" built into Open WebUI.

Concretely:

1. **Security/trust is the main axis.** Build a dedicated rule class (`OWSEC`),
   prioritized by import-time severity (scope-aware: module/`__init__` level vs.
   method body). Coverage: import-time execution, `subprocess`/`eval`/`exec`/
   `os.system`, network, filesystem writes, env/secret access, `requirements`
   supply-chain, unsandboxed `execute` JS.
2. **Trust report as an output format.** Condense security findings into an
   admin-readable risk verdict — the machine implementation of OWUI's "review code
   before importing" and "audit installed plugins regularly".
3. **CI/community workflow as distribution for that story** (GitHub Action,
   pre-commit, SARIF-first, quality-gate mode) — coupled to the security axis rather
   than standing alone.
4. **pip/pipx distribution as a reach multiplier** — afterwards, not as a
   differentiator. No Python object model/API.
5. **Version/compatibility mode** only with a durable contributor commitment (ongoing
   maintenance burden; wrong compatibility knowledge is worse than none).
6. **Python binding deferred** until a concrete adapter exists (community validator,
   server-side import check). Without an integration point = added complexity without
   proportional value.

## Positioning: deliberately outside Open WebUI

The "not endorsed by Open WebUI" status (see README disclaimer) is **not a flaw but
strategic protection**:

- Open WebUI *recommends* "review before import" but, as a platform, cannot enforce it
  without breaking its "arbitrary code by design" principle. An **independent** audit
  tool is the only clean place to do this.
- Deep integration would look like "official policy" and force the tool into
  maintainer-consensus discussions (conservative instead of opinionated).
- Analogous pattern: Bandit vs. CPython, hadolint vs. Docker.

Credibility comes precisely from independence and the freedom to be strict and
opinionated.

## Consequences

**Positive**
- A clearly defensible differentiator, backed by primary sources.
- Serves a second persona (admin/reviewer) that the tool does not address today.
- Reuses the existing `analysis/` architecture; no foreign infrastructure.

**Negative / risks**
- Requires a **more precise, scope-aware AST pass**. The current delimiter scanner
  (`OWUI001` level) is insufficient for security rules — this is the real engineering
  bet.
- Security rules with a high false-positive rate are toxic (the tool gets turned off).
  Conservative defaults and an opt-in `--security` profile are mandatory.
- The scanner is structure-only (no data-flow/taint). Detections that need "value X
  reaches sink Y" — env→external-call exfiltration, "write outside the data dir",
  proving an arg is dynamic — are **rescoped to a presence+scope core**, not promised.
  The full data-flow version is deferred (see `OWUI_SEC.md` §2.1, §4).
- Version/compatibility knowledge goes stale without maintenance and then becomes
  *wrong*.

**Neutral**
- The through-line shifts the product from "linter for extension authors" to "trust
  layer for the Open WebUI plugin ecosystem". Ideas 1 + 5 + 3 are one coherent bet; 2
  and 4 are follow-ups; the binding is ballast.

## Implementation

The detailed technical implementation (AST pass, `OWSEC` rule catalog, trust report,
PoC spike) is described as a separate work plan in [`OWUI_SEC.md`](../../OWUI_SEC.md)
and is deliberately kept out of this ADR.
