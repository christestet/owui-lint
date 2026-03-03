# Valves Reference

**Valves** = admin-configurable settings (via Tools/Functions menus).  
**UserValves** = per-user settings (configurable from chat session).  
Both are optional but highly encouraged. Defined inside any `Pipe`, `Pipeline`, `Filter`, or `Tools` class.

---

## Core Pattern

```python
from pydantic import BaseModel, Field
from typing import Literal

class Filter:  # or Pipe, Tools, Pipeline
    class Valves(BaseModel):
        api_key: str = Field(default="", description="API key for the service")
        temperature: float = Field(default=0.7, ge=0.0, le=2.0, description="Sampling temperature")
        mode: Literal["fast", "balanced", "quality"] = Field(default="balanced", description="Processing mode")
        priority: int = Field(default=0, description="Filter priority (lower = runs first)")
        pass  # Recommended for parser compatibility

    class UserValves(BaseModel):
        show_details: bool = Field(default=False, description="Show detailed output")
        pass

    def __init__(self):
        self.valves = self.Valves()  # Admin values available immediately

    def inlet(self, body: dict, __user__: dict):
        # UserValves accessed via __user__["valves"] (it's a UserValves object, not a dict)
        show = __user__["valves"].show_details        # ✅ Correct
        show = dict(__user__["valves"])["show_details"] # ✅ Also works
        # show = __user__["valves"]["show_details"]    # ❌ Returns default, not actual value!
```

**Key rules:**
- Must inherit from `pydantic.BaseModel`
- Must be nested inside the main class
- Always provide `default` and `description`
- Access admin valves via `self.valves.FIELD_NAME`
- Access user valves via `__user__["valves"].FIELD_NAME`

---

## Special Input Types

### Password (masked)

```python
api_key: str = Field(
    default="",
    description="Your API key",
    json_schema_extra={"input": {"type": "password"}}
)
```

### Select Dropdown — Static Options

```python
log_level: str = Field(
    default="info",
    description="Logging verbosity",
    json_schema_extra={
        "input": {
            "type": "select",
            "options": ["debug", "info", "warn", "error"]
            # Or with labels: [{"value": "debug", "label": "Debug (Verbose)"}, ...]
        }
    }
)
```

### Select Dropdown — Dynamic Options

Pass a `@classmethod` name as a string. It's called when the config UI opens:

```python
class Valves(BaseModel):
    model: str = Field(
        default="",
        description="Choose a model",
        json_schema_extra={"input": {"type": "select", "options": "get_models"}}
    )

    @classmethod
    def get_models(cls, __user__=None) -> list[dict]:
        return [{"value": "gpt-4", "label": "GPT-4"}, {"value": "claude-3", "label": "Claude 3"}]
```

The `__user__` param is optional — include it to generate user-specific options.

---

## Source Code Reference

See **`plugin.py`** → `resolve_valves_schema_options()` for how dynamic options are resolved at runtime.
