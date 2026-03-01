# Pipelines Reference

Pipelines are an **external server framework** that extends Open WebUI by providing OpenAI API-compatible endpoints. They run on a separate server and are connected to Open WebUI via Settings → Connections.

---

## When to Use Pipelines

- Offload heavy processing (GPU inference, large computations) to a separate machine
- Run extensions that need their own dependencies or runtime environment
- Create OpenAI API-compatible endpoints for custom logic
- Advanced multi-service orchestration that shouldn't run on the main instance
- When you need process isolation from the main Open WebUI server

## When NOT to Use Pipelines

- For most use cases, **Functions** (Pipe/Filter/Action) are simpler and preferred
- If you don't need process isolation or separate hardware, use Functions instead
- Pipelines are for advanced users with specific infrastructure needs

## Architecture

```
┌────────────────────┐      HTTP/API      ┌──────────────────┐
│                    │  ◄──────────────►  │                  │
│   Open WebUI       │                    │  Pipelines Server │
│   (Main Instance)  │   OpenAI-compat    │  (Separate Host)  │
│                    │   API format       │                  │
└────────────────────┘                    └──────────────────┘
```

Open WebUI connects to the Pipelines server as if it were an OpenAI-compatible API provider.

## Setting Up Pipelines

### 1. Install the Pipelines Server

```bash
# Using Docker (recommended)
docker run -d \
  -p 9099:9099 \
  --add-host=host.docker.internal:host-gateway \
  -v pipelines:/app/pipelines \
  --name pipelines \
  --restart always \
  ghcr.io/open-webui/pipelines:main

# Or install from source
git clone https://github.com/open-webui/pipelines.git
cd pipelines
pip install -r requirements.txt
python main.py
```

### 2. Connect to Open WebUI

1. Go to **Admin Panel → Settings → Connections**
2. Add the Pipelines server URL (e.g., `http://localhost:9099`)
3. Models from the Pipelines server will appear in the model selector

## Pipeline Pipe Structure

Pipeline Pipes follow a similar pattern to Function Pipes but run on the external server:

```python
from pydantic import BaseModel, Field
from typing import Optional, List, Union, Generator, Iterator

class Pipeline:
    class Valves(BaseModel):
        API_KEY: str = Field(default="", description="API key for external service")
        MODEL_ID: str = Field(default="my-model", description="Model identifier")

    def __init__(self):
        self.name = "My Pipeline"
        self.valves = self.Valves()

    async def on_startup(self):
        """Called when the pipeline server starts."""
        print(f"Pipeline '{self.name}' started")

    async def on_shutdown(self):
        """Called when the pipeline server shuts down."""
        print(f"Pipeline '{self.name}' shut down")

    async def on_valves_updated(self):
        """Called when valve values are updated."""
        print(f"Valves updated for '{self.name}'")

    def pipes(self) -> List[dict]:
        """Return available models/pipes."""
        return [
            {"id": "model-1", "name": "My Model 1"},
            {"id": "model-2", "name": "My Model 2"},
        ]

    def pipe(self, body: dict) -> Union[str, Generator, Iterator]:
        """Process the request and return a response."""
        messages = body.get("messages", [])
        model = body.get("model", "")
        
        # Strip pipeline prefix from model ID
        if "." in model:
            model = model[model.find(".") + 1:]
        
        # Your processing logic
        return f"Response from {model}"
```

## Pipeline Valves

Pipeline Valves work similarly to Function Valves but are configured through the Pipelines server interface or API:

```python
class Pipeline:
    class Valves(BaseModel):
        # Required settings
        API_KEY: str = Field(default="", description="Service API key")
        BASE_URL: str = Field(default="https://api.example.com", description="API base URL")
        
        # Tunable parameters
        TEMPERATURE: float = Field(default=0.7, ge=0.0, le=2.0, description="Sampling temperature")
        MAX_TOKENS: int = Field(default=4096, ge=1, description="Maximum response tokens")
        
        # Feature flags
        STREAM: bool = Field(default=True, description="Enable streaming responses")
        DEBUG: bool = Field(default=False, description="Enable debug logging")

    def __init__(self):
        self.name = "My Pipeline"
        self.valves = self.Valves()
```

## Lifecycle Hooks

Pipelines have lifecycle hooks that Functions don't:

```python
async def on_startup(self):
    """Initialize resources (DB connections, model loading, etc.)."""
    self.client = await create_async_client()

async def on_shutdown(self):
    """Clean up resources."""
    await self.client.close()

async def on_valves_updated(self):
    """React to configuration changes."""
    self.client = await create_async_client(self.valves.BASE_URL)
```

## Full Example: External LLM Proxy Pipeline

```python
from pydantic import BaseModel, Field
from typing import List, Union, Generator, Iterator
import requests
import json

class Pipeline:
    class Valves(BaseModel):
        API_KEY: str = Field(default="", description="Provider API key")
        BASE_URL: str = Field(
            default="https://api.provider.com/v1",
            description="Provider API base URL"
        )
        NAME_PREFIX: str = Field(default="Provider/", description="Model name prefix")

    def __init__(self):
        self.name = "External LLM Proxy"
        self.valves = self.Valves()

    async def on_startup(self):
        print(f"Starting {self.name} pipeline")

    async def on_shutdown(self):
        print(f"Shutting down {self.name} pipeline")

    def pipes(self) -> List[dict]:
        """Fetch available models from the provider."""
        if not self.valves.API_KEY:
            return [{"id": "error", "name": "API Key not configured"}]
        
        try:
            headers = {
                "Authorization": f"Bearer {self.valves.API_KEY}",
                "Content-Type": "application/json",
            }
            r = requests.get(f"{self.valves.BASE_URL}/models", headers=headers)
            r.raise_for_status()
            
            models = r.json().get("data", [])
            return [
                {
                    "id": m["id"],
                    "name": f"{self.valves.NAME_PREFIX}{m.get('name', m['id'])}",
                }
                for m in models
            ]
        except Exception as e:
            return [{"id": "error", "name": f"Error: {e}"}]

    def pipe(self, body: dict) -> Union[str, Generator, Iterator]:
        """Proxy the request to the external provider."""
        headers = {
            "Authorization": f"Bearer {self.valves.API_KEY}",
            "Content-Type": "application/json",
        }

        # Extract actual model ID
        model_id = body["model"]
        if "." in model_id:
            model_id = model_id[model_id.find(".") + 1:]

        payload = {**body, "model": model_id}

        try:
            r = requests.post(
                f"{self.valves.BASE_URL}/chat/completions",
                json=payload,
                headers=headers,
                stream=body.get("stream", False),
            )
            r.raise_for_status()

            if body.get("stream", False):
                return r.iter_lines()
            else:
                return r.json()
        except Exception as e:
            return f"Error: {e}"
```

## Pipeline Examples Repository

The Open WebUI team maintains a collection of example pipelines at:
**https://github.com/open-webui/pipelines/tree/main/examples/pipelines**

These include examples for various providers, RAG setups, and advanced use cases.

## Connection Setup in Open WebUI

1. Deploy the Pipelines server (Docker or standalone)
2. In Open WebUI: **Admin Panel → Settings → Connections**
3. Add the Pipelines server URL (default: `http://localhost:9099`)
4. Models from the Pipelines server appear in the model selector
5. Configure Pipeline Valves through the Pipelines server admin interface
