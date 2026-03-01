# Common Pitfalls, Issues & Troubleshooting

This reference covers the most frequently encountered problems when developing and deploying Open WebUI extensions, along with proven solutions. Read this before building any extension to avoid hours of debugging.

---

## Table of Contents

1. [Architecture & Type Confusion](#1-architecture--type-confusion)
2. [Tool Calling Issues](#2-tool-calling-issues)
3. [Pipe Function Pitfalls](#3-pipe-function-pitfalls)
4. [Filter Function Pitfalls](#4-filter-function-pitfalls)
5. [Streaming Response Issues](#5-streaming-response-issues)
6. [Valves & Configuration Issues](#6-valves--configuration-issues)
7. [Event Emitter Issues](#7-event-emitter-issues)
8. [Execution Order & Lifecycle](#8-execution-order--lifecycle)
9. [Debugging Techniques](#9-debugging-techniques)
10. [Pipeline Server Issues](#10-pipeline-server-issues)
11. [Model Compatibility](#11-model-compatibility)
12. [Security Pitfalls](#12-security-pitfalls)
13. [Performance Issues](#13-performance-issues)

---

## 1. Architecture & Type Confusion

### Problem: Mixing up Pipes, Filters, Actions, and Tools

The most common beginner mistake is using the wrong extension type. Each type has a specific class name that Open WebUI uses to determine how to handle it.

**Critical rule: The class name determines the type.**

| Class Name | Extension Type | Can have inlet/outlet? |
|---|---|---|
| `class Pipe:` | Pipe Function (custom model) | NO — Pipes do NOT support inlet/outlet |
| `class Filter:` | Filter Function (middleware) | YES — this is the ONLY type with inlet/outlet |
| `class Action:` | Action Function (button) | NO |
| `class Tools:` | Workspace Tool (LLM ability) | NO |
| `class Pipeline:` | Pipeline (external server) | YES (on Pipelines server only) |

**Common mistake:**
```python
# WRONG — Pipe class does NOT support inlet/outlet
class Pipe:
    def inlet(self, body, __user__=None):  # This will NEVER be called!
        ...
    def pipe(self, body):
        ...
```

**Fix:** If you need inlet/outlet, use `class Filter:`. If you need a custom model, use `class Pipe:`. You cannot combine them in a single file — Pipes, Filters, and Actions are distinct types and cannot be mixed in a single source file.

### Problem: Not understanding where extensions live

| Type | Location | Who manages |
|---|---|---|
| Workspace Tools | Workspace → Tools | Users with permission |
| Functions (Pipe/Filter/Action) | Admin Panel → Functions | Admins only |
| Pipelines | External server + Settings → Connections | Advanced users |

Extensions installed in the wrong place will not work or won't be visible.

---

## 2. Tool Calling Issues

### Problem: LLM never calls the tool

This is the #1 reported issue. The LLM sees the tool but never invokes it.

**Causes and fixes:**

1. **Docstring is missing or vague.** The LLM reads the function's docstring and parameter descriptions to decide when to use a tool. Without clear, explicit descriptions, the LLM won't know when to call it.

   ```python
   # BAD — no docstring, LLM has no idea what this does
   async def fetch_data(self, url: str) -> str:
       return requests.get(url).text

   # GOOD — explicit docstring tells LLM exactly when and how to use it
   async def fetch_data(self, url: str, __user__: dict = {}) -> str:
       """
       Fetch the content of a web page at the given URL. Use this tool when
       the user asks you to read, scrape, or get information from a website
       or URL. Returns the text content of the page.

       :param url: The full URL to fetch (e.g., "https://example.com/page")
       :return: The text content of the web page
       """
       return requests.get(url).text
   ```

2. **Tool not enabled in chat.** Tools must be explicitly enabled per chat session from the "+" menu in the prompt input box. Even globally enabled tools need to be toggled on per chat.

3. **Function Calling mode mismatch.** Open WebUI has two modes:
   - **Default Mode** (prompt-based): Works with any model, uses a system prompt to guide tool selection. More reliable for smaller/local models.
   - **Native Mode**: Uses the model's built-in function calling. Only works with models fine-tuned for tool use (GPT-4, Claude, Llama 3.1+, Qwen 2.5+).

   Toggle between them in chat advanced settings. If one doesn't work, try the other.

4. **Parameter descriptions missing `Field`.** Tool parameters should use `Field` for descriptions:

   ```python
   # Preferred: use docstring :param notation
   async def search(self, query: str) -> str:
       """
       Search for information.
       :param query: The search query string
       """

   # Alternative: use Field in function signature is NOT directly supported for tools.
   # Use docstrings instead.
   ```

5. **Model doesn't support tool calling well.** Some models (especially small local models) struggle with tool calling. Models known to work well: GPT-4o, Claude, Llama 3.1+, Qwen 2.5+, Command-R. Models with known issues: DeepSeek V3.2 has reproducible native function calling failures — use Default Mode instead.

### Problem: Tool is called but model ignores the result

After the tool executes, the model may produce an empty response or not incorporate the tool result.

**Fixes:**
- Ensure your tool returns a clear, well-structured string that the model can easily incorporate.
- If using Native Mode and the model produces empty responses after tool calls, switch to Default Mode.
- Check if streaming is enabled — some models have issues with tool calling when streaming is off. Test with streaming enabled.

### Problem: Tool calling prompt appears in responses

The model leaks the internal tool calling instructions into its response.

**Fix:** Use Native Mode if your model supports it. In Default Mode, the tool calling prompt is injected as a system message, and weaker models may not properly separate it from their output.

---

## 3. Pipe Function Pitfalls

### Problem: Pipe doesn't appear as a model

After creating and enabling a Pipe Function, it doesn't show in the model selector.

**Fixes:**
- Ensure the function is **enabled** in Admin Panel → Functions.
- Ensure the class is named `Pipe` (not `Pipeline`, not `Filter`).
- If using `pipes()` method (manifold), ensure it returns a valid list of dicts with `id` and `name` keys.
- Check for errors in the `pipes()` method — if it throws an exception, the models won't be listed. Always wrap in try/except.

### Problem: Model ID extraction from manifold pipes

When using manifold pipes (multiple models from one Pipe), Open WebUI prefixes the model ID. You must strip the prefix:

```python
async def pipe(self, body: dict):
    model_id = body["model"]
    # Strip the pipe prefix for manifold pipes
    if "." in model_id:
        model_id = model_id[model_id.find(".") + 1:]
    # Now model_id contains just the actual model identifier
```

**Forgetting this causes:** Sending the wrong model name to external APIs, resulting in 404 or model-not-found errors.

### Problem: Pipe returns raw API response instead of text

When proxying to external APIs, you must extract the text content:

```python
# WRONG — returns full JSON response object
async def pipe(self, body: dict):
    r = requests.post(url, json=body, headers=headers)
    return r.json()  # This shows raw JSON in chat!

# RIGHT for non-streaming — extract the text
async def pipe(self, body: dict):
    r = requests.post(url, json=body, headers=headers)
    data = r.json()
    return data["choices"][0]["message"]["content"]

# RIGHT for streaming — return the line iterator
async def pipe(self, body: dict):
    r = requests.post(url, json=body, headers=headers, stream=True)
    if body.get("stream", False):
        return r.iter_lines()  # Open WebUI handles SSE parsing
    else:
        data = r.json()
        return data["choices"][0]["message"]["content"]
```

---

## 4. Filter Function Pitfalls

### Problem: Filter inlet/outlet never gets called

**Causes:**
1. **Class is not named `Filter`.** It MUST be `class Filter:` — not `class Pipe:`, not `class Pipeline:`.
2. **Filter not assigned to a model.** After enabling, you must either:
   - Assign it to specific models: Workspace → Models → select model → assign filter
   - Enable globally: Workspace → Functions → "..." → toggle Global
3. **Using API instead of WebUI.** Outlet filters are only called when using the WebUI chat interface. Direct API calls to `/api/chat/completions` may not trigger outlet filters.

### Problem: inlet() doesn't return body

The `inlet()` function MUST return the modified `body` dict. If you forget, the body becomes `None` and the request breaks.

```python
# WRONG — missing return statement
async def inlet(self, body: dict, __user__=None) -> dict:
    body["messages"].insert(0, {"role": "system", "content": "Be helpful"})
    # Forgot to return body!

# RIGHT
async def inlet(self, body: dict, __user__=None) -> dict:
    body["messages"].insert(0, {"role": "system", "content": "Be helpful"})
    return body  # ALWAYS return body
```

### Problem: Multiple filters conflict or run in wrong order

Use the `priority` valve to control execution order. Lower values run first:

```python
class Valves(BaseModel):
    priority: int = Field(default=0, description="Filter priority (lower = first)")
```

---

## 5. Streaming Response Issues

### Problem: UI stuck in "executing" / "streaming" state

The chat UI shows the response but never stops the loading indicator.

**Causes and fixes:**

1. **AsyncGenerator not handled correctly.** As of recent versions, returning an `AsyncGenerator` from a pipe can cause the UI to hang. Use synchronous generators or `iter_lines()` from `requests` instead:

   ```python
   # POTENTIALLY PROBLEMATIC — async generator may hang
   async def pipe(self, body: dict):
       async def generate():
           yield "Hello "
           yield "World"
       return generate()

   # SAFER — use requests streaming
   async def pipe(self, body: dict):
       r = requests.post(url, json=body, headers=headers, stream=True)
       return r.iter_lines()
   ```

2. **Missing SSE termination.** When returning streaming data, ensure the stream properly ends. If proxying SSE, the `[DONE]` message must come through.

3. **Blocking synchronous code in async function.** If your `async def pipe()` calls blocking synchronous code (like `requests.post` without streaming), it can block the entire event loop. Use `aiohttp` for async HTTP calls or run blocking code in a thread:

   ```python
   import aiohttp

   async def pipe(self, body: dict):
       async with aiohttp.ClientSession() as session:
           async with session.post(url, json=body, headers=headers) as resp:
               if body.get("stream", False):
                   async for line in resp.content:
                       yield line.decode()
               else:
                   data = await resp.json()
                   return data["choices"][0]["message"]["content"]
   ```

### Problem: Instance becomes very slow during streaming

When using synchronous `requests` library in a pipe with streaming, it can block the web server's event loop, making the entire instance unresponsive.

**Fix:** Use `aiohttp` for streaming responses, or at minimum use `requests` with `stream=True` and return `r.iter_lines()` rather than reading the entire response into memory.

---

## 6. Valves & Configuration Issues

### Problem: Valve changes don't take effect

After updating Valve values in the UI, the extension still uses old values.

**Fixes:**
- Valve values are stored in the database and loaded at runtime. Make sure you're reading `self.valves.FIELD_NAME` inside the function, not caching values in `__init__`.
- For Pipelines, implement `on_valves_updated()` to react to changes.

### Problem: UserValves not accessible

UserValves values are passed through the `__user__` dict, not through `self`:

```python
# WRONG
user_lang = self.user_valves.LANGUAGE

# RIGHT
user_valves = __user__.get("valves", {})
user_lang = user_valves.get("LANGUAGE", "en")
```

### Problem: Valves don't appear in UI

- Ensure the `Valves` class is defined inside your main class and inherits from `BaseModel`.
- Ensure you import `BaseModel` and `Field` from `pydantic`.
- Ensure all fields have explicit types and default values.
- For Pipelines: Valve values are stored in `PIPELINES_DIR/pipeline_name/valves.json`.

### Problem: Sensitive values (API keys) exposed

**Always use Valve Fields for secrets** — never hard-code them. Valve values are stored server-side and not exposed to end users through the chat interface.

```python
# NEVER do this
class Pipe:
    API_KEY = "sk-1234567890abcdef"

# ALWAYS do this
class Pipe:
    class Valves(BaseModel):
        API_KEY: str = Field(default="", description="API key (set by admin)")
```

---

## 7. Event Emitter Issues

### Problem: `__event_emitter__` causes 'NoneType' error

The error `'NoneType' object has no attribute 'get'` when using `__event_emitter__`.

**Fix:** Always check if `__event_emitter__` is not None before calling it:

```python
if __event_emitter__:
    await __event_emitter__({"type": "status", "data": {"description": "Working...", "done": False}})
```

### Problem: Emitted messages not appearing in chat

When using Native function calling with tools, messages emitted via `__event_emitter__` during tool execution may not be displayed in the conversation. This is a known issue in certain Open WebUI versions.

**Workaround:** Include important information in the tool's return value rather than only emitting it as events. Use event emitters for supplementary info (status updates, citations) and return critical data as the function's return string.

### Problem: Status indicator stuck after error

If your function throws an exception after emitting a "working" status, the status indicator stays forever.

**Fix:** Always use try/finally to ensure the "done" status is emitted:

```python
async def pipe(self, body: dict, __event_emitter__=None):
    try:
        if __event_emitter__:
            await __event_emitter__({"type": "status", "data": {"description": "Processing...", "done": False}})
        
        result = await self.do_work(body)
        return result
    except Exception as e:
        return f"Error: {e}"
    finally:
        if __event_emitter__:
            await __event_emitter__({"type": "status", "data": {"description": "Complete", "done": True}})
```

---

## 8. Execution Order & Lifecycle

### Problem: Pipe function executes before inlet in Pipe Functions

In some Open WebUI versions, `pipe()` is called before `inlet()` in Pipe-type Functions. This is a known issue.

**Key clarification:** Pipe Functions (`class Pipe:`) do NOT support `inlet()` and `outlet()`. These hooks belong to Filter Functions (`class Filter:`) only. If you need both custom model logic and input/output modification, create two separate extensions: a Pipe for the model logic and a Filter for input/output processing.

### Problem: Extension dependencies not installed

When installing extensions through the Open WebUI UI, additional Python dependencies are NOT automatically installed. The UI won't show an error if the import fails.

**Fixes:**
- For Docker: Install dependencies in the container, or use `PIPELINES_URLS` environment variable which parses frontmatter and installs dependencies.
- For manual installs: `pip install` the required packages in the same environment.
- Check Docker logs for import errors: `docker logs open-webui`

### Problem: Extensions break after Open WebUI update

Internal APIs (`open_webui.models.*`, `open_webui.utils.*`) can change between versions. Extensions using internal functions may break.

**Fix:**
- Pin your Open WebUI version if stability is critical.
- When using internal functions, wrap calls in try/except with fallback behavior.
- Follow the Open WebUI changelog for breaking changes.
- Prefer using standard HTTP calls to external APIs over internal function imports.

---

## 9. Debugging Techniques

### Problem: No visible output from print() statements

`print()` output goes to the server logs, not the chat interface.

**How to see logs:**
```bash
# Docker
docker logs -f open-webui
docker logs -f pipelines

# Systemd
journalctl -u open-webui -f
```

### Technique: Use event emitters for live debugging

```python
async def pipe(self, body: dict, __event_emitter__=None):
    if __event_emitter__:
        await __event_emitter__({
            "type": "status",
            "data": {"description": f"DEBUG: body keys = {list(body.keys())}", "done": False}
        })
    ...
```

### Technique: Return debug info as response

During development, return debug information directly:

```python
async def pipe(self, body: dict):
    import json
    return f"```json\n{json.dumps(body, indent=2, default=str)}\n```"
```

### Technique: Check model field to verify routing

```python
async def pipe(self, body: dict):
    model = body.get("model", "unknown")
    messages = body.get("messages", [])
    return f"Model: {model}\nMessages: {len(messages)}\nLast: {messages[-1]['content'][:100] if messages else 'none'}"
```

---

## 10. Pipeline Server Issues

### Problem: Pipeline silently fails to load

When installing a pipeline through the Open WebUI UI (not via PIPELINES_URLS), import failures are silent — no error is shown.

**Fix:** Always check the Pipelines server logs:
```bash
docker logs -f pipelines
```

Failed pipeline files are moved to the `failed/` directory inside the pipelines volume.

### Problem: Pipeline installs but models don't appear

**Checklist:**
1. Pipeline server is running and accessible from Open WebUI
2. The connection URL is correct in Settings → Connections
3. The Pipeline class has a `pipes()` method that returns valid model dicts
4. The `pipes()` method doesn't throw (wrap in try/except)
5. Check Pipelines server logs for errors

### Problem: Bricked Open WebUI after pipeline installation

Installing a broken pipeline can crash Open WebUI in some cases.

**Recovery:**
1. Remove the problematic pipeline file from the pipelines volume
2. Restart the containers: `docker restart open-webui pipelines`
3. If still broken, check for duplicate pipeline installations

---

## 11. Model Compatibility

### Models known to work well with tool calling

- GPT-4o, GPT-4o-mini (Native Mode)
- Claude 3.5+, Claude 4+ (Native Mode)
- Llama 3.1+ (70B+ preferred for reliable calling)
- Qwen 2.5+ (good tool calling support)
- Command-R, Command-R+ (Cohere)
- Gemma 3 (reasonable with Default Mode)

### Models with known tool calling issues

- **DeepSeek V3.2**: Known native function calling failures. Use Default Mode.
- **Small local models (<7B)**: Often unreliable with tool calling. Use Default Mode with explicit prompting.
- **DeepSeek-R1**: Reasoning models may overthink tool use. Ollama API endpoint may not pass tool definitions — use OpenAI-compatible endpoint instead.

### Tips for improving tool calling reliability

1. Write extremely explicit docstrings — pretend the model has never seen your tool before.
2. Use Default Mode for models that struggle with Native Mode.
3. Keep tool parameter count low (1-3 parameters).
4. Use simple types (str, int, bool) — avoid complex objects.
5. Return clear, structured text — not raw JSON dumps.
6. Don't offer too many tools at once — fewer tools = better selection.

---

## 12. Security Pitfalls

### Problem: Regular users can upload malicious tools

**Fix:** Restrict Workspace access. Only trusted administrators should have permission to create, import, or modify Tools and Functions. Configure this in Admin Settings → User Permissions.

### Problem: Community extensions contain malicious code

**Fix:** ALWAYS review source code before importing. Check for:
- File system access (`open()`, `os.*`, `shutil.*`)
- Network calls to unknown servers
- Environment variable reading (`os.environ`)
- Subprocess execution (`subprocess.*`, `os.system()`)
- Base64 encoded strings (may hide malicious payloads)

### Problem: API keys leaked in logs or responses

**Fix:**
- Never log `self.valves` directly (it contains API keys)
- Never include API keys in error messages returned to chat
- Use `print(f"API call to {self.valves.BASE_URL}")` instead of `print(f"Using key: {self.valves.API_KEY}")`

---

## 13. Performance Issues

### Problem: Synchronous HTTP calls blocking the event loop

Using `requests` library in `async` functions blocks the entire Open WebUI server.

**Fix:** Use `aiohttp` for async HTTP calls:

```python
import aiohttp

async def pipe(self, body: dict):
    async with aiohttp.ClientSession() as session:
        async with session.post(url, json=body) as resp:
            data = await resp.json()
            return data["choices"][0]["message"]["content"]
```

Or for simpler cases, run synchronous code in a thread executor:

```python
import asyncio
import requests

async def pipe(self, body: dict):
    loop = asyncio.get_event_loop()
    response = await loop.run_in_executor(None, lambda: requests.post(url, json=body))
    return response.json()["choices"][0]["message"]["content"]
```

### Problem: Large context causing slow tool calls

Complex multi-step workflows with 15-30 tool calls can cause "schema drift" where the model's argument formats degrade over time.

**Fix:**
- Keep tool interactions short — aim for 1-3 tool calls per turn.
- Clear or summarize conversation history periodically.
- Use simpler models for tool routing (set a dedicated TASK_MODEL in settings).

### Problem: Memory leaks from unclosed HTTP sessions

**Fix:** Always use context managers or explicitly close sessions:

```python
# GOOD — session is automatically closed
async with aiohttp.ClientSession() as session:
    async with session.post(url) as resp:
        ...

# BAD — session is never closed
session = aiohttp.ClientSession()
resp = await session.post(url)
```

---

## Quick Diagnostic Checklist

When something doesn't work, check these in order:

1. **Is the class name correct?** (Pipe, Filter, Action, Tools, Pipeline)
2. **Is the extension enabled?** (Admin Panel → Functions or Workspace → Tools)
3. **Is it assigned to the right model?** (Filters/Actions need model assignment or Global toggle)
4. **Are there import errors?** (Check server logs: `docker logs open-webui`)
5. **Does the function return the correct type?** (inlet/outlet must return body dict)
6. **Is the docstring descriptive enough?** (For tools — LLM needs to know when to call it)
7. **Are Valves configured?** (API keys, URLs, etc.)
8. **Is the right function calling mode selected?** (Native vs Default)
9. **Are you checking the correct logs?** (`docker logs open-webui` for Functions, `docker logs pipelines` for Pipelines)
10. **Did you try restarting?** (Some changes require a container restart)