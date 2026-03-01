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

### Common Inlet Patterns

**Add system instructions:**
```python
async def inlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
    context = f"Current user: {__user__.get('name', 'Unknown')}. Current time: {datetime.now()}"
    body["messages"].insert(0, {"role": "system", "content": context})
    return body
```

**Validate input length:**
```python
async def inlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
    messages = body.get("messages", [])
    if messages:
        last_msg = messages[-1].get("content", "")
        if len(last_msg) > self.valves.MAX_INPUT_LENGTH:
            raise Exception(f"Message too long. Max {self.valves.MAX_INPUT_LENGTH} characters.")
    return body
```

**Inject RAG context:**
```python
async def inlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
    messages = body.get("messages", [])
    if messages:
        user_query = messages[-1]["content"]
        # Fetch relevant documents
        context = await self.search_documents(user_query)
        # Augment the message
        messages[-1]["content"] = f"Context:\n{context}\n\nUser question: {user_query}"
    body["messages"] = messages
    return body
```

## The `outlet` Function

The outlet intercepts the response **after** the model generates it. Use it to modify, format, or enrich the output.

### Important: `outlet` MUST return the body dict

```python
async def outlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
    messages = body.get("messages", [])
    
    # Modify the last assistant message
    if messages and messages[-1]["role"] == "assistant":
        content = messages[-1]["content"]
        # Example: Add a disclaimer footer
        messages[-1]["content"] = content + "\n\n---\n*This response was AI-generated.*"
    
    body["messages"] = messages
    return body  # ALWAYS return body
```

### Common Outlet Patterns

**Format output:**
```python
async def outlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
    messages = body.get("messages", [])
    if messages and messages[-1]["role"] == "assistant":
        content = messages[-1]["content"]
        # Clean up extra whitespace
        content = "\n".join(line.strip() for line in content.split("\n"))
        messages[-1]["content"] = content
    body["messages"] = messages
    return body
```

**Log conversations:**
```python
async def outlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
    messages = body.get("messages", [])
    if messages:
        user_id = __user__.get("id", "unknown") if __user__ else "unknown"
        print(f"[LOG] User {user_id}: {len(messages)} messages exchanged")
    return body
```

**Translate output:**
```python
async def outlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
    messages = body.get("messages", [])
    if messages and messages[-1]["role"] == "assistant":
        content = messages[-1]["content"]
        translated = await self.translate(content, target_lang=self.valves.TARGET_LANG)
        messages[-1]["content"] = translated
    body["messages"] = messages
    return body
```

## Full Example: Context Injection Filter

```python
from pydantic import BaseModel, Field
from typing import Optional
from datetime import datetime

class Filter:
    class Valves(BaseModel):
        priority: int = Field(default=0, description="Filter priority (lower = first)")
        SYSTEM_PROMPT: str = Field(
            default="You are a helpful assistant. Be concise and accurate.",
            description="System prompt to inject into every conversation"
        )
        ADD_TIMESTAMP: bool = Field(default=True, description="Add timestamp to context")
        ADD_DISCLAIMER: bool = Field(default=True, description="Add disclaimer to output")
        DISCLAIMER_TEXT: str = Field(
            default="\n\n---\n*AI-generated response. Verify important information.*",
            description="Disclaimer text to append to responses"
        )

    def __init__(self):
        self.valves = self.Valves()

    async def inlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
        """Inject system prompt and context before model processing."""
        messages = body.get("messages", [])
        
        # Build system message
        system_parts = [self.valves.SYSTEM_PROMPT]
        
        if self.valves.ADD_TIMESTAMP:
            system_parts.append(f"Current date/time: {datetime.now().strftime('%Y-%m-%d %H:%M')}")
        
        if __user__:
            system_parts.append(f"User: {__user__.get('name', 'Unknown')}")
        
        system_content = "\n".join(system_parts)
        
        # Insert or replace system message
        if messages and messages[0]["role"] == "system":
            messages[0]["content"] = system_content
        else:
            messages.insert(0, {"role": "system", "content": system_content})
        
        body["messages"] = messages
        return body

    async def outlet(self, body: dict, __user__: Optional[dict] = None) -> dict:
        """Add disclaimer to model output."""
        if not self.valves.ADD_DISCLAIMER:
            return body
        
        messages = body.get("messages", [])
        if messages and messages[-1]["role"] == "assistant":
            messages[-1]["content"] += self.valves.DISCLAIMER_TEXT
        
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

## Installation & Activation

1. Go to **Admin Panel → Functions**
2. Click **+** or **Import**
3. Paste or upload the Python code
4. **Enable** the function
5. **Assign to models**: Go to **Workspace → Models**, select a model, and assign the filter
6. **Or enable globally**: In **Workspace → Functions**, click "..." and toggle **Global** to apply to all models
7. Configure Valves as needed
