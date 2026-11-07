"""Quotes frontmatter title/description values in all docs pages.

Unquoted YAML values containing ': ' (for example 'auth: JWT' inside a
description) parse as nested mappings. This wraps every frontmatter
title and description value in double quotes with minimal escaping.
"""

from pathlib import Path
import re

DOCS = Path("/home/z/my-project/docs-site/src/content/docs")


def quote_value(value: str) -> str:
    value = value.strip()
    already = len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}
    if already:
        return value
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


for path in sorted(DOCS.rglob("*")):
    if path.suffix not in {".md", ".mdx"}:
        continue
    text = path.read_text(encoding="utf-8")
    match = re.match(r"^---\n(.*?)\n---\n", text, flags=re.DOTALL)
    if not match:
        continue
    front = match.group(1)
    new_lines = []
    changed = False
    for line in front.split("\n"):
        field = re.match(r"^(title|description):\s*(.+)$", line)
        if field:
            quoted = f"{field.group(1)}: {quote_value(field.group(2))}"
            if quoted != line:
                changed = True
            new_lines.append(quoted)
        else:
            new_lines.append(line)
    if changed:
        new_text = text[: match.start(1)] + "\n".join(new_lines) + text[match.end(1) :]
        path.write_text(new_text, encoding="utf-8")
        print(f"quoted: {path.relative_to(DOCS)}")

print("done")
