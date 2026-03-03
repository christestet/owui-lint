# Action Functions Reference

Action Functions add **custom buttons** to the chat interface that appear under individual chat messages.

---

## When to Use Actions

- Add a "Summarize" button to condense long messages
- Add a "Translate" button for quick translation
- Add a "Copy as Markdown" or "Export" button
- Add a "Fact Check" button that verifies claims
- Add a "Read Aloud" button for text-to-speech
- Any one-click operation on a specific chat message

## Basic Structure

```python
from pydantic import BaseModel, Field
from typing import Optional, Callable

class Action:
    class Valves(BaseModel):
        ENABLED: bool = Field(default=True, description="Enable this action")

    def __init__(self):
        self.valves = self.Valves()

    async def action(
        self,
        body: dict,
        __user__: Optional[dict] = None,
        __event_emitter__: Callable = None,
    ) -> Optional[dict]:
        """Called when the user clicks the action button."""
        # body contains the message and context
        # Use __event_emitter__ to send results back to the UI
        
        message_content = body.get("messages", [])[-1].get("content", "")
        
        # Process the message
        result = f"Processed: {message_content[:100]}..."
        
        # Emit the result
        if __event_emitter__:
            await __event_emitter__({
                "type": "message",
                "data": {"content": result}
            })
```

## The `body` Dictionary for Actions

When a user clicks an action button, the body contains:

```python
{
    "messages": [
        # The full conversation history up to and including
        # the message the button was clicked on
        {"role": "user", "content": "What is quantum computing?"},
        {"role": "assistant", "content": "Quantum computing is..."}
    ],
    "model": "current-model-id",
    # ... other request parameters
}
```

## Using Event Emitters in Actions

Actions primarily communicate results via `__event_emitter__` (e.g., status updates, citations, or appending messages).

👉 **See [development-common.md](development-common.md) for full examples of Status, Message, and Citation events.**

## Full Example: Translate Action

This example adds a translation button. Instead of calling an external API, it intercepts the click, modifies the request body to ask the current model for a translation, and returns the modified body to Open WebUI to process.

```python
from pydantic import BaseModel, Field
from typing import Optional, Callable

class Action:
    class Valves(BaseModel):
        TARGET_LANGUAGE: str = Field(default="Spanish", description="Language to translate to")

    def __init__(self):
        self.valves = self.Valves()

    async def action(
        self,
        body: dict,
        __event_emitter__: Callable = None,
    ) -> Optional[dict]:
        """Translate the clicked message using the current model."""
        messages = body.get("messages", [])
        if not messages: return

        target_message = messages[-1].get("content", "")
        
        if __event_emitter__:
            await __event_emitter__({"type": "status", "data": {"description": f"Translating to {self.valves.TARGET_LANGUAGE}...", "done": False}})
        
        # Modify the body to ask the current model to translate
        body["messages"] = [
            {"role": "system", "content": f"Translate the following text to {self.valves.TARGET_LANGUAGE}. Only output the translation, nothing else."},
            {"role": "user", "content": target_message}
        ]
        
        if __event_emitter__:
            await __event_emitter__({"type": "status", "data": {"description": "Done!", "done": True}})
        
        return body
```

## Source Code Reference

For exact runtime behavior — how `action()` is invoked, which reserved args are injected, sub-action routing (`action_id.sub_action_id`), and Rich UI embed processing — see **`actions.py`** in this references directory. Key details:

- Reserved args injected: `__model__`, `__id__`, `__event_emitter__`, `__event_call__`, `__request__`
- `__user__` is passed as a dict (via `user.model_dump()`), with `UserValves` merged in if defined
- Both sync and async `action()` are supported (detected via `inspect.iscoroutinefunction`)
- Action results are processed through `process_tool_result` for Rich UI embeds

---

## Installation & Activation

1. Go to **Admin Panel → Functions**
2. Click **+** or **Import**
3. Paste or upload the Python code
4. **Enable** the function
5. **Assign to models**: Go to **Workspace → Models**, select a model, and assign the action
6. **Or enable globally**: In **Workspace → Functions**, click "..." and toggle **Global**
7. The button will appear under chat messages for assigned models
8. Configure Valves (click gear icon) for API keys and settings
