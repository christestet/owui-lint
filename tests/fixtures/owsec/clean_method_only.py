"""
title: Clean method-scoped subprocess
version: 0.1.0

Negative fixture: the same dangerous calls, but only inside a tool method that
runs when the LLM invokes the tool. OWSEC001 must NOT fire here.
"""

import subprocess

from pydantic import BaseModel


class Tools:
    class Valves(BaseModel):
        pass

    def __init__(self):
        self.valves = self.Valves()

    async def ping(self, host: str) -> str:
        """Ping a host (runs only when the tool is called)."""
        result = subprocess.run(["ping", "-c", "1", host], capture_output=True)
        return result.stdout.decode()
