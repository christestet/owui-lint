# Development Common: Valves, Events, Reserved Args, Rich UI

This reference covers shared concepts that apply to ALL Open WebUI extension types (Tools, Pipes, Filters, Actions, Pipelines).

---

## Table of Contents

1. [Valves — Persistent Configuration](#valves)
2. [UserValves — Per-User Configuration](#uservalves)
3. [Reserved Arguments](#reserved-arguments)
4. [Events — Emitting UI Events](#events)
5. [Rich UI Embedding](#rich-ui-embedding)

---

## Valves & UserValves

**Valves** (admin-configurable settings) and **UserValves** (per-user settings) are highly recommended for all extensions to avoid hardcoding API keys, URLs, and tunable parameters.

👉 **See [valves.md](valves.md) for the complete reference**, including how to define them, how to mask passwords, and how to create dynamic dropdown menus.

## Reserved Arguments

Open WebUI injects special arguments into your functions. Add them as parameters to receive them automatically.

### Available in ALL extension types

| Argument | Type | Description |
|---|---|---|
| `__user__` | `dict` | Current user info: `{"id": "...", "email": "...", "name": "...", "role": "...", "valves": {...}}` |
| `__metadata__` | `dict` | Request metadata including chat_id, message_id, session_id, etc. |
| `__request__` | `Request` | FastAPI Request object (for internal API calls) |
| `__event_emitter__` | `Callable` | Function to emit events back to the UI (status updates, citations, etc.) |
| `__event_call__` | `Callable` | Function to call events and await responses |

### Usage Example

```python
async def pipe(
    self,
    body: dict,
    __user__: dict,
    __event_emitter__: Callable,
    __request__: Request,
) -> str:
    user_name = __user__.get("name", "Unknown")
    chat_id = __metadata__.get("chat_id", "")
    
    await __event_emitter__({"type": "status", "data": {"description": "Processing..."}})
    
    return f"Hello, {user_name}!"
```

### Important Notes

- Only include the arguments you need — Open WebUI detects them by name
- `__user__` is the most commonly used
- `__event_emitter__` is essential for streaming status updates
- `__request__` is needed when calling internal Open WebUI functions like `generate_chat_completion`

---

## Events

Events allow your extension to communicate with the UI in real-time. Use `__event_emitter__` to send events.

### Status Events

Show processing status to the user:

```python
# Show a status message (appears as a loading indicator)
await __event_emitter__({
    "type": "status",
    "data": {
        "description": "Fetching data from API...",
        "done": False
    }
})

# Mark status as complete
await __event_emitter__({
    "type": "status",
    "data": {
        "description": "Complete!",
        "done": True
    }
})
```

### Message Events

Send additional messages or content:

```python
# Send a message to the chat
await __event_emitter__({
    "type": "message",
    "data": {
        "content": "Here is some additional information..."
    }
})
```

### Citation Events

Add source citations to responses:

```python
await __event_emitter__({
    "type": "citation",
    "data": {
        "document": ["Content of the source..."],
        "metadata": [{"source": "https://example.com/article"}],
        "source": {"name": "Example Article", "url": "https://example.com/article"}
    }
```

---

## Rich UI Embedding

You can embed interactive HTML/JavaScript content directly in chat responses using a special code block syntax.

### How It Works

Return a fenced code block with `html` language tag in your response. Open WebUI will render it as an interactive iframe:

```python
async def pipe(self, body: dict):
    html_content = """
    <div style="padding: 20px; font-family: sans-serif;">
        <h2>Interactive Chart</h2>
        <canvas id="myChart" width="400" height="200"></canvas>
        <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
        <script>
            const ctx = document.getElementById('myChart').getContext('2d');
            new Chart(ctx, {
                type: 'bar',
                data: {
                    labels: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri'],
                    datasets: [{
                        label: 'Activity',
                        data: [12, 19, 3, 5, 2]
                    }]
                }
            });
        </script>
    </div>
    """
    return f"Here is your chart:\n\n```html\n{html_content}\n```"
```

### Important Notes

- The HTML runs inside a sandboxed iframe
- External scripts from CDNs are supported
- Keep the HTML self-contained — avoid dependencies on the parent page
- Useful for charts, interactive forms, visualizations, mini-apps

---

## Source Code References

For exact implementation details on the concepts above, see these Python source files in this references directory:

| Concept | Source File | What to Look For |
|---|---|---|
| Valves loading & schema | `plugin.py` | `resolve_valves_schema_options()`, Valves initialization in `load_function_module_by_id()` / `load_tool_module_by_id()` |
| Reserved args injection | `tools.py`, `filter.py`, `actions.py` | `extra_params` dict construction, `inspect.signature()` checks for which args are passed |
| Event emitter/call setup | `actions.py` | `get_event_emitter()` and `get_event_call()` with chat_id, message_id, session_id, user_id |
| Rich UI embed processing | `actions.py` | `process_tool_result()` call and embed event emission |
| UserValves loading | `filter.py`, `actions.py`, `tools.py` | `Functions.get_user_valves_by_id_and_user_id()` merged into `__user__` dict |
