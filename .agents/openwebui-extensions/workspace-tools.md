# Workspace Tools Reference

Workspace Tools are **Python scripts that give LLMs new abilities** during conversations. The LLM can invoke these tools to fetch data, perform calculations, interact with APIs, and more.

---

## When to Use Workspace Tools

- Let the LLM fetch real-time data (weather, stocks, exchange rates)
- Let the LLM call external APIs (search engines, databases, services)
- Let the LLM perform complex calculations or data processing
- Let the LLM interact with files, web scraping, or other I/O operations
- Any capability you want the LLM to autonomously decide to use during chat

## Key Difference from Functions

- **Tools** = abilities the LLM uses during conversation (the LLM decides when to call them)
- **Functions** (Pipe/Filter/Action) = platform-level extensions (modify the UI, routing, or processing)

## Basic Structure

```python
from pydantic import BaseModel, Field
from typing import Optional

class Tools:
    class Valves(BaseModel):
        API_KEY: str = Field(default="", description="API key for the service")

    class UserValves(BaseModel):
        PREFERRED_UNIT: str = Field(default="metric", description="Preferred unit system")

    def __init__(self):
        self.valves = self.Valves()

    async def my_tool_function(
        self,
        parameter1: str,
        parameter2: int = 10,
        __user__: dict = {},
    ) -> str:
        """
        Description of what this tool does. This docstring is shown to the LLM
        so it knows WHEN and HOW to use this tool.
        
        :param parameter1: Description of parameter1 (shown to LLM)
        :param parameter2: Description of parameter2 (shown to LLM)
        :return: Description of what is returned
        """
        # Your tool logic here
        return f"Result for {parameter1}"
```

## Critical: Docstrings Matter

The LLM reads the **function name**, **docstring**, and **parameter descriptions** to decide when and how to call your tool. Write clear, descriptive docstrings:

```python
async def get_weather(
    self,
    city: str,
    units: str = "metric",
    __user__: dict = {},
) -> str:
    """
    Get the current weather for a given city. Use this when the user asks
    about weather conditions, temperature, humidity, or forecast for a
    specific location.
    
    :param city: The city name (e.g., "London", "New York", "Tokyo")
    :param units: Unit system - "metric" for Celsius, "imperial" for Fahrenheit
    :return: Current weather conditions as a formatted string
    """
```

## Parameter Types

The LLM will try to fill parameters based on conversation context:

```python
# String parameters
async def search(self, query: str) -> str:

# Integer parameters
async def get_top_n(self, query: str, count: int = 5) -> str:

# Boolean parameters
async def fetch_data(self, url: str, include_headers: bool = False) -> str:

# Optional parameters
async def lookup(self, name: str, category: Optional[str] = None) -> str:
```

## Using Event Emitters in Tools

Tools can emit status updates and citations:

```python
async def web_search(
    self,
    query: str,
    __user__: dict = {},
    __event_emitter__: Callable = None,
) -> str:
    """Search the web for information."""
    
    # Show status
    if __event_emitter__:
        await __event_emitter__({
            "type": "status",
            "data": {"description": f"Searching for: {query}", "done": False}
        })
    
    results = await self._perform_search(query)
    
    # Add citations
    for result in results:
        if __event_emitter__:
            await __event_emitter__({
                "type": "citation",
                "data": {
                    "document": [result["snippet"]],
                    "metadata": [{"source": result["url"]}],
                    "source": {"name": result["title"], "url": result["url"]}
                }
            })
    
    # Mark done
    if __event_emitter__:
        await __event_emitter__({
            "type": "status",
            "data": {"description": "Search complete!", "done": True}
        })
    
    return "\n".join(r["snippet"] for r in results)
```

## Full Example: Weather Tool

```python
from pydantic import BaseModel, Field
from typing import Callable, Optional
import requests

class Tools:
    class Valves(BaseModel):
        OPENWEATHER_API_KEY: str = Field(default="", description="OpenWeatherMap API key")
        DEFAULT_UNITS: str = Field(default="metric", description="Default unit system (metric/imperial)")

    class UserValves(BaseModel):
        PREFERRED_UNITS: str = Field(default="metric", description="Your preferred unit system")

    def __init__(self):
        self.valves = self.Valves()

    async def get_current_weather(
        self,
        city: str,
        __user__: dict = {},
        __event_emitter__: Callable = None,
    ) -> str:
        """
        Get the current weather conditions for a city. Use this tool when the user
        asks about current weather, temperature, humidity, wind, or conditions for
        any location.
        
        :param city: City name (e.g., "London", "Berlin, DE", "Tokyo, JP")
        :return: Current weather information including temperature, conditions, humidity
        """
        if not self.valves.OPENWEATHER_API_KEY:
            return "Error: OpenWeatherMap API key not configured. Please set it in the tool's Valves."
        
        # Get user's preferred units
        user_valves = __user__.get("valves", {})
        units = user_valves.get("PREFERRED_UNITS", self.valves.DEFAULT_UNITS)
        
        if __event_emitter__:
            await __event_emitter__({
                "type": "status",
                "data": {"description": f"Fetching weather for {city}...", "done": False}
            })
        
        try:
            url = "https://api.openweathermap.org/data/2.5/weather"
            params = {
                "q": city,
                "appid": self.valves.OPENWEATHER_API_KEY,
                "units": units,
            }
            
            r = requests.get(url, params=params)
            r.raise_for_status()
            data = r.json()
            
            temp_unit = "°C" if units == "metric" else "°F"
            speed_unit = "m/s" if units == "metric" else "mph"
            
            weather = (
                f"Weather in {data['name']}, {data['sys']['country']}:\n"
                f"- Conditions: {data['weather'][0]['description'].title()}\n"
                f"- Temperature: {data['main']['temp']}{temp_unit} "
                f"(feels like {data['main']['feels_like']}{temp_unit})\n"
                f"- Humidity: {data['main']['humidity']}%\n"
                f"- Wind: {data['wind']['speed']} {speed_unit}"
            )
            
            if __event_emitter__:
                await __event_emitter__({
                    "type": "status",
                    "data": {"description": "Weather data retrieved!", "done": True}
                })
            
            return weather
        
        except requests.exceptions.HTTPError as e:
            return f"Error fetching weather data: {e}"
        except Exception as e:
            return f"Unexpected error: {e}"

    async def get_forecast(
        self,
        city: str,
        days: int = 3,
        __user__: dict = {},
        __event_emitter__: Callable = None,
    ) -> str:
        """
        Get a multi-day weather forecast for a city. Use this when the user asks
        about upcoming weather, future conditions, or weather planning.
        
        :param city: City name (e.g., "London", "New York")
        :param days: Number of days to forecast (1-5, default 3)
        :return: Weather forecast for the requested number of days
        """
        # Implementation similar to above but using /forecast endpoint
        return f"Forecast for {city} ({days} days): [implementation here]"
```

## Multiple Tools in One Class

You can define multiple tool functions in a single `Tools` class:

```python
class Tools:
    class Valves(BaseModel):
        API_KEY: str = Field(default="")

    def __init__(self):
        self.valves = self.Valves()

    async def search_web(self, query: str) -> str:
        """Search the web for information."""
        ...

    async def get_stock_price(self, symbol: str) -> str:
        """Get current stock price for a ticker symbol."""
        ...

    async def calculate(self, expression: str) -> str:
        """Evaluate a mathematical expression."""
        ...
```

Each function becomes a separate tool the LLM can call.

## Installation & Activation

1. Go to **Workspace → Tools**
2. Click **+** or **Import**
3. Paste or upload the Python code
4. The tool becomes available to assign to models
5. In chat, when selecting a model, enable the tool from the tools panel
6. Configure Valves (click gear icon) for API keys and settings
7. Users can configure UserValves from their own settings

## Security Warning

⚠️ **Workspace Tools execute arbitrary Python on your server.** Only trusted users with explicit permission should be allowed to create or import tools. Review all code before installation.
