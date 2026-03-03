---
name: openwebui-extensions
description: "Create Open WebUI extensions: Tools, Functions (Pipe, Filter, Action), and Pipelines. Use this skill whenever a user wants to extend Open WebUI, create custom models/agents, add buttons to chat, filter messages, integrate external APIs, build workspace tools, or set up pipeline servers. Triggers on keywords like 'open webui', 'openwebui', 'pipe function', 'filter function', 'action function', 'workspace tool', 'open webui plugin', 'open webui pipeline', 'open webui extension', 'open webui custom model', 'manifold pipe', or when users describe problems that involve extending LLM capabilities within Open WebUI (e.g., 'I want my chatbot to fetch weather data', 'I need a custom button in my chat', 'I want to add Anthropic as a model provider'). Also trigger when users mention modifying chat input/output, creating proxy models, or offloading processing to external servers."
---

# Open WebUI Extensions Builder

This skill helps users create extensions for Open WebUI by analyzing their problem and determining the correct extension type, then generating production-ready Python code.


## Relevant References

Before writing any code, read the appropriate reference file:

| Extension Type | Reference File | When to Read |
|---|---|---|
| Workspace Tool | `references/workspace-tools.md` | User wants to give LLM new abilities during chat |
| Pipe Function | `references/pipe-functions.md` | User wants a custom model/agent in the selector |
| Filter Function | `references/filter-functions.md` | User wants to modify input/output of existing models |
| Action Function | `references/action-functions.md` | User wants custom buttons on chat messages |
| Pipeline | `references/pipelines.md` | User needs external server processing |
| Valves & UserValves | `references/valves.md` | Defining configurable settings (admin Valves, per-user UserValves, input types, dynamic options) |
| Valves, Events & Reserved Args | `references/development-common.md` | Always read — covers Valves, Events, Reserved Args, Rich UI |
| Pitfalls & Troubleshooting | `references/pitfalls-and-troubleshooting.md` | Always read — covers common mistakes, debugging, model compatibility |

### Source Code References (Open WebUI Backend)

These are actual Python source files from the Open WebUI codebase. Read them when you need exact function signatures, parameter handling, class detection, or runtime behavior.

| Source File | What It Covers | When to Read |
|---|---|---|
| `references/plugin.py` | Module loading (`load_function_module_by_id`, `load_tool_module_by_id`), frontmatter extraction, import replacement, Valves schema resolution, caching | Understanding how extensions are loaded, how class names (`Pipe`/`Filter`/`Action`/`Tools`) are detected, how frontmatter requirements work |
| `references/tools.py` | Tool execution (`get_tools`), built-in tools (`get_builtin_tools`), spec generation, tool server integration, access control | Understanding tool loading, function-to-spec conversion, reserved parameter injection (`__user__`, `__event_emitter__`, etc.) |
| `references/filter.py` | Filter processing (`process_filter_functions`), filter ordering/priority, inlet/outlet/stream handler execution | Understanding how filters are chained, how `inlet()`/`outlet()` are called, `file_handler` behavior |
| `references/actions.py` | Action execution (`chat_action`), sub-action routing, event emitter/call setup | Understanding how `action()` is invoked, which reserved args are passed, Rich UI embed processing |

**ALWAYS read `references/development-common.md` AND `references/pitfalls-and-troubleshooting.md` in addition to the type-specific reference.** The common reference contains critical information about Valves, Events, Reserved Args, and Rich UI. The pitfalls reference covers the most frequently encountered issues (wrong class names, silent failures, streaming hangs, tool calling problems) and their solutions — reading it before coding prevents most debugging sessions.

#### Official Documentation & Source URLs

If the local reference files lack detail for an edge case, or if you need to verify against the latest API, fetch the relevant official doc or source file. **Use local references first** — only fetch URLs when you need additional depth.

| Topic | URL |
|-------|-----|
| **Core Extension Docs** | |
| Tools & Functions overview | https://docs.openwebui.com/features/extensibility/plugin/ |
| Functions overview | https://docs.openwebui.com/features/extensibility/plugin/functions/ |
| Pipe Functions | https://docs.openwebui.com/features/extensibility/plugin/functions/pipe |
| Filter Functions | https://docs.openwebui.com/features/extensibility/plugin/functions/filter |
| Action Functions | https://docs.openwebui.com/features/extensibility/plugin/functions/action |
| Tools (Workspace) | https://docs.openwebui.com/features/extensibility/plugin/tools/ |
| Tool Development | https://docs.openwebui.com/features/extensibility/plugin/tools/development |
| **Development APIs** | |
| Events (event_emitter, event_call) | https://docs.openwebui.com/features/extensibility/plugin/development/events |
| Valves & UserValves | https://docs.openwebui.com/features/extensibility/plugin/development/valves |
| Rich UI Embedding | https://docs.openwebui.com/features/extensibility/plugin/development/rich-ui |
| Reserved Args (__user__, __request__) | https://docs.openwebui.com/features/extensibility/plugin/development/reserved-args |
| **Pipelines** | |
| Pipelines overview | https://docs.openwebui.com/features/extensibility/pipelines/ |
| Pipelines: Pipes | https://docs.openwebui.com/features/extensibility/pipelines/pipes |
| Pipelines: Valves | https://docs.openwebui.com/features/extensibility/pipelines/valves |
| Pipelines examples (GitHub) | https://github.com/open-webui/pipelines/tree/main/examples/pipelines |
| **Source Code References (GitHub)** | |
| Functions parsing module | https://raw.githubusercontent.com/open-webui/open-webui/refs/heads/main/backend/open_webui/functions.py |
| Functions Pydantic model | https://raw.githubusercontent.com/open-webui/open-webui/refs/heads/main/backend/open_webui/models/functions.py |
| Tools Pydantic model | https://raw.githubusercontent.com/open-webui/open-webui/refs/heads/main/backend/open_webui/models/tools.py |
| Pipelines main code | https://raw.githubusercontent.com/open-webui/pipelines/refs/heads/main/main.py |
| **Other** | |
| Troubleshooting | https://docs.openwebui.com/troubleshooting/ |

### Implementation Principles

Before writing code, ALWAYS review `references/pitfalls-and-troubleshooting.md`. It covers the most frequently encountered issues and reading it upfront prevents most debugging sessions. 

Follow these core implementation principles:

1. **Use async functions** — Define `pipe()`, `inlet()`, `outlet()`, `action()` as `async`. However, when making HTTP calls inside them, use `aiohttp`. Only use synchronous `requests` for streaming (`stream=True` + `iter_lines()`).
2. **Handle errors gracefully** — Use try/except blocks. Return meaningful error messages to the user.
3. **Include docstrings and comments** — Especially for Tools, as the LLM relies entirely on the docstring to know how to use it.
4. **Follow the class structure order** — Valves → UserValves → `__init__` → main function(s).

## Quick Reference: Extension Types Summary

| Type | Purpose | Appears As | Managed In | Who Creates |
|---|---|---|---|---|
| **Workspace Tool** | Give LLM new abilities (web scraping, API calls, calculations) | Tool the LLM can call during chat | Workspace > Tools | Users (with permission) |
| **Pipe Function** | Custom model/agent or API proxy | Selectable model in UI | Admin Panel > Functions | Admins |
| **Filter Function** | Modify input before model / output after model | Invisible middleware | Admin Panel > Functions | Admins |
| **Action Function** | Custom button on chat messages | Button under messages | Admin Panel > Functions | Admins |
| **Pipeline** | External server for heavy processing | OpenAI API-compatible endpoint | Settings > Connections | Advanced users |