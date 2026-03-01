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

## Valves

Valves are **persistent, admin-configurable settings** for your extension. They survive restarts and are shared across all users.

### Basic Pattern

```python
from pydantic import BaseModel, Field

class Pipe:  # or Tool, Filter, etc.
    class Valves(BaseModel):
        API_KEY: str = Field(default="", description="API key for the service")
        BASE_URL: str = Field(default="https://api.example.com", description="Base API URL")
        MAX_RESULTS: int = Field(default=10, description="Maximum number of results to return")
        ENABLED: bool = Field(default=True, description="Enable or disable this extension")

    def __init__(self):
        self.valves = self.Valves()
```

### Key Rules

- Valves class MUST be defined inside the main class (Pipe, Tools, Filter, etc.)
- Valves MUST inherit from `pydantic.BaseModel`
- Always provide `default` values and `description` for each field
- Initialize with `self.valves = self.Valves()` in `__init__`
- Access values via `self.valves.FIELD_NAME`
- Admins configure Valves via the UI (Admin Panel → Functions or Workspace → Tools)

### Supported Field Types

```python
class Valves(BaseModel):
    # Strings
    API_KEY: str = Field(default="")
    
    # Numbers
    TEMPERATURE: float = Field(default=0.7, ge=0.0, le=2.0)
    MAX_TOKENS: int = Field(default=1000, ge=1, le=100000)
    
    # Booleans
    STREAM: bool = Field(default=True)
    
    # Enums (dropdown in UI)
    MODEL: str = Field(default="gpt-4", enum=["gpt-4", "gpt-3.5-turbo", "claude-3"])
    
    # Lists
    ALLOWED_DOMAINS: list[str] = Field(default=["example.com"])
```

---

## UserValves

UserValves are **per-user configurable settings** that individual users can modify for their own sessions.

```python
class Tools:  # or Pipe, Filter, etc.
    class Valves(BaseModel):
        API_KEY: str = Field(default="", description="Admin API key")

    class UserValves(BaseModel):
        PREFERRED_LANGUAGE: str = Field(default="en", description="User's preferred language")
        SHOW_DETAILS: bool = Field(default=True, description="Show detailed output")

    def __init__(self):
        self.valves = self.Valves()
```

Access UserValves via the `__user__` reserved argument:

```python
async def pipe(self, body: dict, __user__: dict):
    user_valves = __user__.get("valves", {})
    language = user_valves.get("PREFERRED_LANGUAGE", "en")
```

---

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
})
```

### Usage Pattern

```python
async def pipe(self, body: dict, __event_emitter__: Callable):
    # Show progress
    await __event_emitter__({"type": "status", "data": {"description": "Starting...", "done": False}})
    
    # Do work...
    result = await self.fetch_data()
    
    # Add citation
    await __event_emitter__({
        "type": "citation",
        "data": {
            "document": [result["text"]],
            "metadata": [{"source": result["url"]}],
            "source": {"name": result["title"], "url": result["url"]}
        }
    })
    
    # Mark done
    await __event_emitter__({"type": "status", "data": {"description": "Done!", "done": True}})
    
    return result["summary"]
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
