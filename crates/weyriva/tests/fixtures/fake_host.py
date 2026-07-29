#!/usr/bin/env python3
import json
import sys


PROTOCOL = "weyriva-luau-host/1"


def emit(value):
    sys.stdout.write(json.dumps(value, separators=(",", ":")) + "\n")
    sys.stdout.flush()


emit({"protocol": PROTOCOL, "event": "ready"})

for line in sys.stdin:
    request = json.loads(line)
    method = request["method"]
    params = request["params"]
    if method == "shutdown":
        result = {"exit_callback_called": True, "actions": []}
    elif method == "activate":
        result = {
            "activated": params["id"],
            "actions": [{"type": "set_query", "query": ""}],
        }
    elif method == "query":
        query = params["query"]
        if query == "crash":
            sys.exit(7)
        elif query == "large":
            result = {"padding": "x" * (70 * 1024)}
        elif query == "oversize":
            result = {"padding": "x" * (1024 * 1024)}
        else:
            result = {
                "query": query,
                "results": [{
                    "id": "row-1",
                    "title": f"Result {query}",
                    "subtitle": "Fixture result",
                    "glyph": "search",
                    "category": "General",
                }],
                "actions": [],
            }
    else:
        result = {}
    emit({
        "protocol": PROTOCOL,
        "id": request["id"],
        "result": result,
        "error": None,
    })
    if method == "shutdown":
        break
