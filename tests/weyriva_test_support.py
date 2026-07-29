from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def qml_sources() -> dict[Path, str]:
    roots = (ROOT / "shell", ROOT / "greeter")
    return {
        path.relative_to(ROOT): path.read_text()
        for root in roots
        for path in root.rglob("*.qml")
    }


def qml_blocks(source: str, type_name: str) -> list[str]:
    """Return balanced blocks for a QML type without depending on filenames."""
    blocks: list[str] = []
    pattern = re.compile(rf"\b{re.escape(type_name)}\s*\{{")
    for match in pattern.finditer(source):
        depth = 0
        quoted = False
        escaped = False
        for index in range(match.end() - 1, len(source)):
            character = source[index]
            if escaped:
                escaped = False
                continue
            if character == "\\" and quoted:
                escaped = True
                continue
            if character == '"':
                quoted = not quoted
                continue
            if quoted:
                continue
            if character == "{":
                depth += 1
            elif character == "}":
                depth -= 1
                if depth == 0:
                    blocks.append(source[match.start():index + 1])
                    break
    return blocks
