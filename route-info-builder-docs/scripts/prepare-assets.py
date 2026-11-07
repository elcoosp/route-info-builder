"""Copies the project's logo PNG unchanged and derives the PNG favicon.

The canonical brand asset is docs/logo.png in the route-info-builder
repository (1264x1264 RGBA). This script:

1. copies the PNG byte-identically into src/assets/logo.png (no re-encode,
   no lossy optimization, no upscaling);
2. writes public/favicon.png as a 512x512 resample of the original
   (Lanczos, PNG, lossless encoding).

The multicolor mark (orange + dark-gray links) cannot be recolored without
damage, so the same unchanged file serves both themes; dark-mode legibility
is handled in CSS (src/styles/custom.css). See docs/decisions/0002-logo-format.md.
"""

from pathlib import Path
from PIL import Image
import shutil

REPO_LOGO = Path("/home/z/my-project/repo/route-info-builder/docs/logo.png")
SITE = Path("/home/z/my-project/docs-site")

ASSETS = SITE / "src" / "assets"
PUBLIC = SITE / "public"


def main() -> None:
    ASSETS.mkdir(parents=True, exist_ok=True)
    PUBLIC.mkdir(parents=True, exist_ok=True)

    # 1. Byte-identical copy of the canonical PNG.
    logo_dst = ASSETS / "logo.png"
    shutil.copyfile(REPO_LOGO, logo_dst)
    if REPO_LOGO.read_bytes() != logo_dst.read_bytes():
        raise SystemExit("copy is not byte-identical — aborting")
    with Image.open(logo_dst) as logo:
        print(f"logo.png copied unchanged: {logo.size} {logo.mode}")

    # 2. Favicon: 512x512 resample of the original (never upscaled).
    with Image.open(logo_dst) as src:
        if src.width != src.height:
            raise SystemExit("logo is not square; refusing to guess a crop")
        if src.width < 512:
            raise SystemExit("logo is smaller than 512px; refusing to upscale")
        favicon = src.resize((512, 512), Image.LANCZOS)
    favicon_path = PUBLIC / "favicon.png"
    favicon.save(favicon_path, format="PNG", optimize=True)
    with Image.open(favicon_path) as check:
        print(f"favicon.png written: {check.size} {check.mode}")


if __name__ == "__main__":
    main()
