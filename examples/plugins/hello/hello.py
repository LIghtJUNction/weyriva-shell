#!/usr/bin/env python3
"""Harmless Weyriva example plugin."""

import json
import sys


def main() -> int:
    params = json.load(sys.stdin)
    name = params.get("name", "world") if isinstance(params, dict) else "world"
    json.dump({"message": f"Hello, {name}!"}, sys.stdout)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
