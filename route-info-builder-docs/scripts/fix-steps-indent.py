"""Re-indents the <Steps> block of the tutorial so every continuation line
of a numbered item is indented at least three spaces.

Two-space continuation lines (after the tab normalization) fall below the
ordered-list continuation indent and split the list, which trips MD029.
This shifts every line that starts with exactly two spaces inside the
<Steps> ... </Steps> range to four spaces.
"""

from pathlib import Path

TARGET = Path("/home/z/my-project/docs-site/src/content/docs/tutorials/getting-started.mdx")

lines = TARGET.read_text(encoding="utf-8").split("\n")
inside = False
changed = 0
for i, line in enumerate(lines):
    if line.strip() == "<Steps>":
        inside = True
        continue
    if line.strip() == "</Steps>":
        inside = False
        continue
    if inside and line.startswith("  ") and not line.startswith("    "):
        lines[i] = "  " + line
        changed += 1

TARGET.write_text("\n".join(lines), encoding="utf-8")
print(f"re-indented {changed} lines")
