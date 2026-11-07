"""Converts hard tabs to two spaces in the docs content files.

markdownlint (MD010) rejects hard tabs; Starlight/MDX content is authored
with tabs for editor consistency, so this normalizes every *.md/*.mdx under
src/content/docs to two-space indentation without changing semantics.
"""

from pathlib import Path

DOCS = Path("/home/z/my-project/docs-site/src/content/docs")

changed = 0
for path in sorted(DOCS.rglob("*")):
    if path.suffix not in {".md", ".mdx"}:
        continue
    text = path.read_text(encoding="utf-8")
    if "\t" not in text:
        continue
    path.write_text(text.replace("\t", "  "), encoding="utf-8")
    changed += 1
    print(f"normalized: {path.relative_to(DOCS)}")

print(f"{changed} file(s) normalized")
