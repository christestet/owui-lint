# Event Functions Reference

Event Functions react to **system activity** across all of Open WebUI — a signup, a deleted chat, a config change, server startup — instead of shaping a single chat request. **New in Open WebUI 0.10.0.**

---

## When to Use Events

- Run setup/teardown logic on server startup or shutdown (self-applying configuration)
- Gate or auto-hold new signups (email-domain allowlist, burst/bot defense)
- Send welcome content, auto-assign groups, or seed starter content for new users
- Fan out alerts (Slack/Discord/SIEM) when a high-signal event fires (e.g. someone becomes admin)
- Ship an immutable audit log of every event to an external sink
- Self-clean chats/knowledge past an age cutoff, or cascade cleanup on account deletion
- Self-install / self-configure a plugin: register its own API routes on startup and provision itself via admin endpoints
- Anything that should run *because something happened in Open WebUI*, not because a user clicked something in a chat

> **Not for validating or blocking a request.** An Event function runs **after** the
> triggering activity already happened (the user is already created, the message
> already exists) — it can react, remediate, or notify, but it cannot intercept or
> rewrite the request in-band. To transform a chat request as it flows, use a
> [Filter](filter-functions.md) instead.

## Basic Structure

The type is auto-detected from a top-level `class Event`:

```python
"""
title: Example Event
author: you
version: 0.1
"""

from pydantic import BaseModel


class Event:
    class Valves(BaseModel):
        pass

    def __init__(self):
        self.valves = self.Valves()

    async def event(
        self,
        event: dict,
        __event_id__: str = None,
        __event_name__: str = None,
        __id__: str = None,
        __app__=None,
        __request__=None,
    ):
        print(f"event id:   {__event_id__}")
        print(f"event name: {__event_name__}")
        print(f"payload:    {event}")
```

Your `event()` handler is called for **every** system event — you decide what to act on by checking `__event_name__`.

### The `event()` handler arguments

Reserved args are only injected if they appear in the handler's signature (or the handler declares `**kwargs`) — Open WebUI inspects the signature and passes just what's asked for.

| Argument | Type | Description |
|---|---|---|
| `event` | `dict` | The event payload: what changed, plus an `actor` (the user who triggered it, when applicable). Sensitive fields (passwords, tokens, secrets, API keys) are redacted before the payload reaches your function. |
| `__event_name__` | `str` | The event's name, namespaced `<area>.<action>`, e.g. `auth.signup` or `chat.deleted`. Use this to filter. |
| `__event_id__` | `str` | A unique id for this specific event occurrence. |
| `__id__` | `str` | The id of your Event function. |
| `__app__` | `FastAPI` | The Open WebUI FastAPI application instance. Lets you register new API routes, read/update application state, and call internal services. |
| `__request__` | `Request` \| `None` | The current request, when the event was triggered by one (lifecycle events like `system.startup.completed` have no request → `None`). |

> A function whose module lacks an `event` method entirely is **silently skipped** by
> the dispatcher — no error, no log entry, nothing happens. See [Common Mistakes](#common-mistakes) (`OWE600`).

## Filtering by Event Name

```python
async def event(self, event: dict, __event_name__: str = None, **kwargs):
    if __event_name__ == "auth.signup":
        await self.on_signup(event)
    elif __event_name__ == "system.startup.completed":
        await self.on_startup()
```

Declaring `**kwargs` alongside the args you actually use is a convenient way to stay forward-compatible if Open WebUI adds reserved args later, without having to accept every one by name.

## The Event Catalog

Open WebUI emits **170+ events** spanning every subsystem, each namespaced as `<area>.<action>`:

| Area | Example events |
|---|---|
| **System** | `system.startup.started`, `system.startup.completed`, `system.shutdown.started`, `system.shutdown.completed` |
| **Auth** | `auth.signup`, `auth.login`, `auth.logout`, `auth.password_changed`, `auth.api_key.created` |
| **Users & groups** | `user.created`, `user.deleted`, `user.role_updated`, `user.permissions_updated`, `group.member_added` |
| **Chats & messages** | `chat.created`, `chat.deleted`, `chat.shared`, `chat.archived`, `chat.compacted`, `message.created`, `message.reaction_added` |
| **Models & prompts** | `model.created`, `model.updated`, `model.access_updated`, `prompt.created` |
| **Knowledge & files** | `knowledge.created`, `knowledge.file.added`, `knowledge.reindexed`, `file.uploaded`, `file.deleted` |
| **Plugins** | `tool.created`, `function.enabled`, `function.valves_updated`, `tool.valves_updated`, `skill.created` |
| **Config** | `config.updated`, `config.models.updated`, `config.connections.updated`, `config.webhook.updated` |
| **Other** | `channel.*`, `calendar.*`, `automation.*`, `memory.*`, `note.*`, `image.*`, `audio.*`, `terminal.*`, `feedback.*` |

The authoritative, complete list is shown in **Admin Settings > Events** when configuring webhooks. This is a different (but related) mechanism — see [Event Functions vs. Event Webhooks](#event-functions-vs-event-webhooks).

## Sync vs. Async

Open WebUI dispatches events on an `asyncio` event loop and detects `event()` via `inspect.iscoroutinefunction`, so **both sync and async handlers are supported** — but a sync handler runs **inline in the dispatch loop and blocks it** while it executes. A slow or blocking sync handler on a frequent event (e.g. `message.created`) stalls dispatch for every other Event function until it returns.

```python
# ❌ Blocks the async dispatch loop while the request is in flight
def event(self, event: dict, __event_name__: str = None):
    requests.post(self.valves.WEBHOOK_URL, json=event)

# ✅ Non-blocking
async def event(self, event: dict, __event_name__: str = None):
    async with aiohttp.ClientSession() as session:
        await session.post(self.valves.WEBHOOK_URL, json=event)
```

Always declare `event()` as `async` and use an async HTTP client (`aiohttp`) for network calls — see [Common Mistakes](#common-mistakes) (`OWE601`).

## Valves

`Valves` work the same way as other extension types, with one difference: they are **re-instantiated on every dispatch**, not just once at import. Open WebUI re-reads the saved values and rebuilds `self.valves = self.Valves(**saved)` before each `event()` call, so a Valve change made from **Admin Panel → Functions → ⚙️** takes effect on the *next* event without a restart.

```python
class Event:
    class Valves(BaseModel):
        WEBHOOK_URL: str = ""
        ENABLED: bool = True

    def __init__(self):
        self.valves = self.Valves()

    async def event(self, event: dict, __event_name__: str = None):
        if not self.valves.ENABLED:
            return
        # self.valves reflects the latest saved values for this dispatch
```

The `function.valves_updated` event also fires whenever an admin saves new Valve values, so a function can react to its own configuration changing.

👉 **See [valves.md](valves.md) for the complete Valves/UserValves reference.**

## Lifecycle Events

`system.startup.completed` fires once the server is ready; `system.shutdown.started` fires when it begins to stop. Use them for setup/teardown:

```python
async def event(self, event: dict, __event_name__: str = None, __app__=None, **kwargs):
    if __event_name__ == "system.startup.completed":
        # apply saved configuration, warm a cache, register routes, etc.
        ...
    elif __event_name__ == "system.shutdown.started":
        # flush buffers, close connections, etc.
        ...
```

This is what makes Event functions a good fit for **self-applying configuration**: the same setup runs on every startup using values from `Valves`, so the whole configuration lives inside Open WebUI instead of an external script.

> **Multi-replica warning:** lifecycle events fire on **every replica** (each boots and
> shuts down independently), while request-scoped events (`auth.signup`, `chat.deleted`,
> etc.) fire once on the instance that handled the request. One-time provisioning work
> triggered from `system.startup.completed` needs a distributed lock (e.g. Redis
> `SET NX EX`) or it runs once per replica, not once globally. Open WebUI dispatches
> fire-and-forget with **no retry and no cross-replica coordination** — deduplication is
> the function's responsibility.

## Security Considerations

Event functions are a **high-trust primitive** — treat them the same way `OWSEC` treats any entry class, but with a broader trigger surface:

- **Import-time risk is unchanged.** `plugin.py`'s `load_function_module_by_id` instantiates `module.Event()` right after `exec()`-ing the module body, exactly like `Tools`/`Pipe`/`Filter`/`Action` — module-level code and `__init__` (and the `Valves()` it constructs) run at import, before any admin has clicked anything.
- **The trigger is automatic, not user-initiated.** Once installed and enabled, `event()` runs on **every** system event without any further action — there is no "assign to a model" or "click a button" gate the way there is for Pipes, Filters, and Actions. Enabling the function is the only consent step.
- **`__app__` is effectively unrestricted.** It hands the handler the live FastAPI application: register routes, mutate app state, call internal services. Combined with automatic triggering, a malicious or buggy Event function can self-install additional attack surface (new routes) without further admin interaction.
- **Function creation is admin-only.** This is the primary control — only install Event functions from trusted sources and review the code before importing, same as any other function type.
- **Sensitive-field redaction is best-effort, not a security boundary.** The `event` payload has known sensitive keys (passwords, tokens, secrets, API keys) stripped before it reaches your handler, but don't assume the payload is safe to log or forward verbatim — review what a specific event's `data`/`actor` fields contain before shipping them to an external sink.

## Common Mistakes

| Mistake | Why it matters | Rule |
|---|---|---|
| Defining `class Event` without an `event` method | The dispatcher does `getattr(function_module, 'event', None)`; if it's `None` the function is **silently skipped** for every event, forever — no error surfaces anywhere | `OWE600` (error) |
| Defining `event()` as a plain `def` instead of `async def` | Sync handlers run **inline** in the async dispatch loop and block it for the duration of the call, delaying every other Event function's dispatch | `OWE601` (warning) |
| Omitting the `event` parameter from `event()`'s signature | Reserved args are injected only if they appear in the signature (or `**kwargs` is present); a handler that only takes `__event_name__` never sees the payload even though Open WebUI offers it | `OWE602` (warning) |
| Doing slow/blocking work (network calls, heavy compute) synchronously inside `event()` | Same root cause as `OWE601` — even an `async def` can still block the loop if it calls a synchronous, long-running function without `await`ing an async equivalent | — |
| One-time provisioning logic on `system.startup.completed` with no dedup | Fires once **per replica**, not once globally — a naive "send startup alert" runs N times in an N-replica deployment | — |
| Assuming `__request__` is always present | It's `None` for lifecycle events (`system.startup.*`, `system.shutdown.*`), which aren't triggered by an HTTP request — guard for `None` before using it | — |
| Treating `event()` as a place to block/reject the triggering action | Event functions run **after** the fact (the user/chat/message already exists) — they can only react, not intercept; use a [Filter](filter-functions.md) to validate/transform a request in flight | — |

## Event Functions vs. Event Webhooks

Two ways to react to events, for different needs:

- **Event functions** (this page) run Python in-process. Use them for real logic: branching, calling internal services, registering endpoints, transforming data.
- **Event webhooks** (configured in **Admin Settings > Events**) send selected events to external HTTP endpoints as JSON, or to chat destinations (Slack, Discord), filtered by event pattern and by user/group — no code required.

> **Don't confuse with `__event_emitter__`.** This page is about the `Event` **function
> type** (`class Event`, reacts to *system* events like `auth.signup`). That's unrelated
> to the `__event_emitter__`/`__event_call__` reserved args available in Tools, Pipes,
> Filters, and Actions, which push *UI* events (status updates, citations, dialogs) to
> the chat frontend — see [development-common.md](development-common.md#events) for those.

## Source Code Reference

Event functions were added in Open WebUI **0.10.0**, after this skill's bundled `plugin.py`/`filter.py`/`tools.py`/`actions.py` snapshots were pinned (see [`SOURCES.md`](SOURCES.md) — pinned at `v0.9.6`), so the backend `events.py` module isn't bundled locally yet. For exact runtime behavior, fetch it directly: <https://raw.githubusercontent.com/open-webui/open-webui/refs/heads/main/backend/open_webui/events.py>. Key details (cite by symbol, not line — verify against the fetched copy):

- `dispatch_event_functions()`: loads all active functions of type `event` and, for each, does `getattr(function_module, 'event', None)` — `continue`s (skips) if absent
- Valves: re-instantiated per dispatch — `function_module.Valves(**(valves if valves else {}))` — before the handler is called
- Reserved-arg injection: builds an `extra_params` dict (`event`, `__id__`, `__event__`, `__event_id__`, `__event_name__`, `__app__`, `__request__`) and filters it down to keys present in `inspect.signature(handler).parameters`, unless the handler accepts `**kwargs` (`VAR_KEYWORD`), in which case all of them are passed
- Dispatch: `inspect.iscoroutinefunction(handler)` picks `await handler(**params)` vs. a plain blocking `handler(**params)` call, inline in the loop
- Exceptions from a handler are caught and logged per-function; one failing Event function does not stop dispatch to the others
- Loader: `plugin.py`'s `load_function_module_by_id` returns `module.Event(), 'event', frontmatter` — the same `exec()` + immediate-instantiation pattern as `Tools`/`Pipe`/`Filter`/`Action` (see this directory's `plugin.py`, pre-0.10.0 snapshot, for that shared pattern)

Official docs: <https://docs.openwebui.com/features/extensibility/plugin/functions/event> ("New in 0.10.0").

---

## Installation & Activation

1. Go to **Admin Panel → Functions**
2. Click **+** or **Import**
3. Paste or upload the Python code
4. **Enable** the function — `event()` now runs automatically on every subsequent system event; there is no per-model assignment step (Event functions aren't tied to a chat request)
5. Configure Valves (click the gear icon) for endpoints, allowlists, schedules, API keys, and toggles
6. Only admins can create/import functions — review the code before enabling, since it now runs unattended in response to normal server activity
