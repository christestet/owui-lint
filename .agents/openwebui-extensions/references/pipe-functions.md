# Pipe Functions Reference

Pipe Functions create **custom models/agents** that appear in the Open WebUI model selector. They are the most versatile extension type.

---

## When to Use Pipes

- Proxy requests to external LLM providers (Anthropic, Azure, Google, etc.)
- Create custom agents with multi-step logic
- Integrate non-AI services as "models" (search engines, home automation, databases)
- Combine outputs from multiple models into one response
- Create a manifold (one Pipe → multiple models in the selector)

## Basic Structure

```python
from pydantic import BaseModel, Field
from typing import Callable, Optional

class Pipe:
    class Valves(BaseModel):
        API_KEY: str = Field(default="", description="API key for the service")

    def __init__(self):
        self.valves = self.Valves()

    async def pipe(self, body: dict, __user__: dict) -> str:
        """Process input and return response."""
        # body contains: model, messages, stream, and other OpenAI-compatible fields
        messages = body.get("messages", [])
        user_message = messages[-1]["content"] if messages else ""
        
        # Your logic here
        return "Hello from my custom pipe!"
```

## The `body` Dictionary

The `body` parameter follows the OpenAI chat completion format:

```python
{
    "model": "my-pipe-model-id",
    "messages": [
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello!"},
        {"role": "assistant", "content": "Hi there!"},
        {"role": "user", "content": "What's the weather?"}
    ],
    "stream": True,  # or False
    "temperature": 0.7,
    "max_tokens": 1000
}
```

## Return Types

### Simple String Response

```python
async def pipe(self, body: dict) -> str:
    return "This is my response"
```

### Streaming Response

For streaming, return an iterator or generator:

```python
async def pipe(self, body: dict):
    if body.get("stream", False):
        return self.stream_response(body)
    else:
        return "Non-streaming response"

def stream_response(self, body):
    """Generator that yields chunks."""
    for chunk in ["Hello ", "from ", "streaming!"]:
        yield chunk
```

Or when proxying to an external API:

```python
import requests

async def pipe(self, body: dict):
    r = requests.post(
        url=f"{self.valves.BASE_URL}/chat/completions",
        json=body,
        headers={"Authorization": f"Bearer {self.valves.API_KEY}"},
        stream=True,
    )
    r.raise_for_status()
    
    if body.get("stream", False):
        return r.iter_lines()
    else:
        return r.json()
```

## Manifold: Multiple Models from One Pipe

Define a `pipes()` method to register multiple models:

```python
class Pipe:
    class Valves(BaseModel):
        API_KEY: str = Field(default="", description="API key")

    def __init__(self):
        self.valves = self.Valves()

    def pipes(self):
        """Return list of models this pipe provides."""
        return [
            {"id": "model-fast", "name": "Fast Model"},
            {"id": "model-smart", "name": "Smart Model"},
            {"id": "model-creative", "name": "Creative Model"},
        ]

    async def pipe(self, body: dict):
        # Extract which model was selected
        model_id = body["model"]
        # For manifold pipes, strip the pipe prefix
        if "." in model_id:
            model_id = model_id[model_id.find(".") + 1:]
        
        # Route to the appropriate backend
        if model_id == "model-fast":
            return await self.call_fast_model(body)
        elif model_id == "model-smart":
            return await self.call_smart_model(body)
        else:
            return await self.call_creative_model(body)
```

### Dynamic Model Discovery

The `pipes()` method can fetch models dynamically from an API:

```python
def pipes(self):
    if not self.valves.API_KEY:
        return [{"id": "error", "name": "API Key not provided."}]
    
    try:
        headers = {"Authorization": f"Bearer {self.valves.API_KEY}"}
        r = requests.get(f"{self.valves.BASE_URL}/models", headers=headers)
        models = r.json()
        return [
            {"id": m["id"], "name": f"MyProvider/{m['id']}"}
            for m in models["data"]
        ]
    except Exception as e:
        return [{"id": "error", "name": f"Error: {e}"}]
```

## Full Example: Anthropic Proxy Pipe

```python
from pydantic import BaseModel, Field
import requests

class Pipe:
    class Valves(BaseModel):
        ANTHROPIC_API_KEY: str = Field(default="", description="Anthropic API key")

    def __init__(self):
        self.valves = self.Valves()

    def pipes(self):
        return [{"id": "claude-sonnet", "name": "Anthropic/Claude Sonnet"}]

    async def pipe(self, body: dict, __user__: dict):
        if not self.valves.ANTHROPIC_API_KEY:
            return "Error: API key not set."

        # Convert OpenAI format to Anthropic format
        messages = [{"role": m["role"], "content": m["content"]} for m in body.get("messages", []) if m["role"] != "system"]
        system_msg = next((m["content"] for m in body.get("messages", []) if m["role"] == "system"), "")

        payload = {
            "model": "claude-3-5-sonnet-20241022",
            "messages": messages,
            "max_tokens": 4096,
        }
        if system_msg: payload["system"] = system_msg

        try:
            r = requests.post(
                "https://api.anthropic.com/v1/messages",
                json=payload,
                headers={"x-api-key": self.valves.ANTHROPIC_API_KEY, "anthropic-version": "2023-06-01"},
            )
            r.raise_for_status()
            return r.json()["content"][0]["text"]
        except Exception as e:
            return f"Error: {e}"
```

## Using Internal Open WebUI Functions

You can call Open WebUI's own internal functions to route through existing models:

```python
from fastapi import Request
from open_webui.models.users import Users
from open_webui.utils.chat import generate_chat_completion

class Pipe:
    def __init__(self):
        pass

    async def pipe(self, body: dict, __user__: dict, __request__: Request):
        user = Users.get_user_by_id(__user__["id"])
        body["model"] = "llama3.2:latest"  # Route to a specific model
        return await generate_chat_completion(__request__, body, user)
```

## Source Code Reference

For exact runtime behavior — how pipe modules are loaded, class detection, and caching — see these Python source files in this references directory:

- **`plugin.py`**: `load_function_module_by_id()` shows how `class Pipe:` is detected and instantiated, how frontmatter requirements are installed, and how modules are cached via `get_function_module_from_cache()`
- **`tools.py`**: `get_tool_specs()` and `convert_function_to_pydantic_model()` show how function signatures are converted to OpenAI-compatible tool specs (relevant if your pipe also exposes tools)

---

## Installation & Activation

1. Go to **Admin Panel → Functions**
2. Click **+** or **Import**
3. Paste or upload the Python code
4. **Enable** the function — it will appear as a selectable model in the chat interface
5. Configure Valves (click the gear icon) to set API keys etc.
