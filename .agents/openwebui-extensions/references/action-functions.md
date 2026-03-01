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

Actions primarily communicate results via `__event_emitter__`:

### Send a text message back to chat

```python
await __event_emitter__({
    "type": "message",
    "data": {"content": "Here is the summary: ..."}
})
```

### Show processing status

```python
await __event_emitter__({
    "type": "status",
    "data": {"description": "Summarizing...", "done": False}
})

# ... do work ...

await __event_emitter__({
    "type": "status",
    "data": {"description": "Done!", "done": True}
})
```

### Add citations

```python
await __event_emitter__({
    "type": "citation",
    "data": {
        "document": ["Source text..."],
        "metadata": [{"source": "https://example.com"}],
        "source": {"name": "Source Name", "url": "https://example.com"}
    }
})
```

## Full Example: Summarize Action

```python
from pydantic import BaseModel, Field
from typing import Optional, Callable
import requests

class Action:
    class Valves(BaseModel):
        OPENAI_API_KEY: str = Field(default="", description="OpenAI API key for summarization")
        OPENAI_BASE_URL: str = Field(
            default="https://api.openai.com/v1",
            description="OpenAI API base URL"
        )
        SUMMARY_MODEL: str = Field(default="gpt-4o-mini", description="Model to use for summaries")
        MAX_SUMMARY_LENGTH: int = Field(default=150, description="Target summary length in words")

    def __init__(self):
        self.valves = self.Valves()

    async def action(
        self,
        body: dict,
        __user__: Optional[dict] = None,
        __event_emitter__: Callable = None,
    ) -> Optional[dict]:
        """Summarize the clicked message."""
        
        messages = body.get("messages", [])
        if not messages:
            return
        
        # Get the message to summarize
        target_message = messages[-1].get("content", "")
        
        if not target_message:
            if __event_emitter__:
                await __event_emitter__({
                    "type": "message",
                    "data": {"content": "No content to summarize."}
                })
            return
        
        # Show processing status
        if __event_emitter__:
            await __event_emitter__({
                "type": "status",
                "data": {"description": "Generating summary...", "done": False}
            })
        
        try:
            # Call OpenAI API for summarization
            headers = {
                "Authorization": f"Bearer {self.valves.OPENAI_API_KEY}",
                "Content-Type": "application/json",
            }
            payload = {
                "model": self.valves.SUMMARY_MODEL,
                "messages": [
                    {
                        "role": "system",
                        "content": f"Summarize the following text in approximately {self.valves.MAX_SUMMARY_LENGTH} words. Be concise and capture the key points."
                    },
                    {"role": "user", "content": target_message}
                ],
                "max_tokens": 500,
            }
            
            r = requests.post(
                f"{self.valves.OPENAI_BASE_URL}/chat/completions",
                json=payload,
                headers=headers,
            )
            r.raise_for_status()
            
            summary = r.json()["choices"][0]["message"]["content"]
            
            if __event_emitter__:
                await __event_emitter__({
                    "type": "message",
                    "data": {"content": f"**Summary:**\n\n{summary}"}
                })
                await __event_emitter__({
                    "type": "status",
                    "data": {"description": "Summary complete!", "done": True}
                })
        
        except Exception as e:
            if __event_emitter__:
                await __event_emitter__({
                    "type": "message",
                    "data": {"content": f"Error generating summary: {e}"}
                })
                await __event_emitter__({
                    "type": "status",
                    "data": {"description": "Error", "done": True}
                })
```

## Full Example: Simple Translate Action (No External API)

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
        __user__: Optional[dict] = None,
        __event_emitter__: Callable = None,
    ) -> Optional[dict]:
        """Translate the clicked message using the current model."""
        
        messages = body.get("messages", [])
        if not messages:
            return
        
        target_message = messages[-1].get("content", "")
        
        if __event_emitter__:
            await __event_emitter__({
                "type": "status",
                "data": {"description": f"Translating to {self.valves.TARGET_LANGUAGE}...", "done": False}
            })
        
        # Instead of calling an external API, we modify the body to ask
        # the current model to translate. Return the modified body.
        body["messages"] = [
            {
                "role": "system",
                "content": f"Translate the following text to {self.valves.TARGET_LANGUAGE}. Only output the translation, nothing else."
            },
            {"role": "user", "content": target_message}
        ]
        
        if __event_emitter__:
            await __event_emitter__({
                "type": "status",
                "data": {"description": "Done!", "done": True}
            })
        
        return body
```

## Installation & Activation

1. Go to **Admin Panel → Functions**
2. Click **+** or **Import**
3. Paste or upload the Python code
4. **Enable** the function
5. **Assign to models**: Go to **Workspace → Models**, select a model, and assign the action
6. **Or enable globally**: In **Workspace → Functions**, click "..." and toggle **Global**
7. The button will appear under chat messages for assigned models
8. Configure Valves (click gear icon) for API keys and settings
