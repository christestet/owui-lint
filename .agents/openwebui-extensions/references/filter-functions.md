# Filter Functions Reference

Filter Functions act as **middleware** that modify data before it reaches the model (inlet) and/or after the model responds (outlet).

---

## When to Use Filters

- Add system instructions or context to every message automatically
- Clean up, format, or transform model output
- Translate input/output between languages
- Censor or redact sensitive information
- Log conversations for auditing
- Add metadata or timestamps
- Rate-limit or validate user input
- Inject RAG context before the model sees the message

## Basic Structure

```python
from pydantic import BaseModel, Field
from typing import Optional, Callable

class Filter:
    class Valves(BaseModel):
        ENABLED: bool = Field(default=True, description="Enable this filter")
        priority: int = Field(default=0, description="Filter priority (lower = runs first)")

    def __init__(self):
        self.valves = self.Valves()

    async def inlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
        """Modify the request BEFORE it reaches the model."""
        # body contains the full request (messages, model, etc.)
        # MUST return the modified body dict
        return body

    async def outlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
        """Modify the response AFTER the model generates it."""
        # body contains the full response
        # MUST return the modified body dict
        return body
```

## The `inlet` Function

The inlet intercepts the request **before** it goes to the model. Use it to modify user input, add context, or validate messages.

### Important: `inlet` MUST return the body dict

```python
async def inlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
    messages = body.get("messages", [])
    
    # Example: Prepend a system message
    system_msg = {"role": "system", "content": "Always respond in formal English."}
    if messages and messages[0]["role"] != "system":
        messages.insert(0, system_msg)
    
    body["messages"] = messages
    return body  # ALWAYS return body
```

### Common Patterns

**Validate input length (Inlet):**
```python
async def inlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
    if messages := body.get("messages", []):
        if len(messages[-1].get("content", "")) > self.valves.MAX_LENGTH:
            raise Exception("Message too long.")
    return body
```

**Clean output (Outlet):**
```python
async def outlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
    if messages := body.get("messages", []):
        if messages[-1]["role"] == "assistant":
            messages[-1]["content"] = messages[-1]["content"].strip()
    return body
```

## Full Example: Context Injection Filter

```python
from pydantic import BaseModel, Field
from typing import Optional

class Filter:
    class Valves(BaseModel):
        priority: int = Field(default=0, description="Filter priority (lower = first)")
        SYSTEM_PROMPT: str = Field(default="You are a helpful assistant.", description="System prompt to inject")

    def __init__(self):
        self.valves = self.Valves()

    async def inlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
        """Inject system prompt before model processing."""
        messages = body.get("messages", [])
        system_content = f"{self.valves.SYSTEM_PROMPT}\nUser: {__user__.get('name', '') if __user__ else 'Unknown'}"
        
        if messages and messages[0]["role"] == "system":
            messages[0]["content"] = system_content
        else:
            messages.insert(0, {"role": "system", "content": system_content})
            
        body["messages"] = messages
        return body

    async def outlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
        """Add disclaimer to model output."""
        messages = body.get("messages", [])
        if messages and messages[-1]["role"] == "assistant":
            messages[-1]["content"] += "\n\n*AI-generated response.*"
            
        body["messages"] = messages
        return body
```

## Filter Priority

When multiple filters are active, the `priority` valve controls execution order:

- **Lower number = runs first**
- Default is 0
- Use negative numbers to run before others
- Use positive numbers to run after others

```python
class Valves(BaseModel):
    priority: int = Field(default=0, description="Filter priority")
```

## Source Code Reference

For exact runtime behavior — how filters are chained, priority sorting, handler dispatch — see **`filter.py`** in this references directory. Key details:

- Filters are sorted by `(priority, filter_id)` via `get_sorted_filter_ids()`
- Three handler types: `inlet`, `outlet`, `stream` (stream handlers skip DB reload for performance)
- Handler receives `body` as param name for inlet/outlet, `event` for stream
- Reserved args (`__id__`, `__user__`, etc.) are injected only if present in the handler signature
- `file_handler` attribute: if set on the filter module and handler is `inlet`, file metadata is removed from the body after processing
- Valves are loaded from DB and applied before each filter runs
- Both sync and async handlers are supported

---

## Installation & Activation

1. Go to **Admin Panel → Functions**
2. Click **+** or **Import**
3. Paste or upload the Python code
4. **Enable** the function
5. **Assign to models**: Go to **Workspace → Models**, select a model, and assign the filter
6. **Or enable globally**: In **Workspace → Functions**, click "..." and toggle **Global** to apply to all models
7. Configure Valves as needed
