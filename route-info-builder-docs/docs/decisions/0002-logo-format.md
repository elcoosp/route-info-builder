# 0002 — Logo format, theme variants and brand accent

Date: 2026-11-08

## Source asset

- Canonical logo: `docs/logo.png` in the route-info-builder repository, referenced by the
  project's own `README.md`.
- Verified properties: 1264×1264 px, RGBA, transparent background.
- Copy into `src/assets/logo.png` is **byte-identical** (asserted by
  `scripts/prepare-assets.py`, which refuses to finish otherwise). No re-encode, no
  lossy optimization, no upscaling anywhere.

## Light/dark variants

The mark is multicolor: orange links (≈#bb6d21) intertwined with dark-gray links on
transparency. Per the decision tree:

- Recoloring would damage the mark (a channel negate would destroy the orange), so the
  skill's "recolor would damage the mark" branch applies: **the same unchanged PNG serves
  both themes** (`logo: { src: './src/assets/logo.png' }`, no `replacesTitle` — the mark
  contains no project name text).
- The dark-gray links would be invisible on Starlight's dark background, so dark-mode
  legibility is solved in CSS only: a light rounded backdrop applied to
  `[data-theme='dark'] .site-title img` and `[data-theme='dark'] .hero img` in
  `src/styles/custom.css`. Pixels are never edited.

## Favicon

`public/favicon.png` is the original PNG resampled to 512×512 with Lanczos (PNG, lossless
encoding). No SVG favicon, no `.ico`. Registered as `favicon: '/favicon.png'`.

## Accent color

- Sampled dominant saturated orange of the mark: **#bb6d21** (hsl(30 70% 43%)).
- Light mode uses `--sl-color-accent: hsl(30 70% 37%)` (#a05e1c): the sampled value
  measured 3.94:1 with white text (below WCAG AA 4.5:1 for body text), the deepened value
  measures 5.08:1 — same hue, AA-compliant at any text size.
- Dark mode lightens the family: accent hsl(32 75% 58%), accent-high hsl(34 85% 78%).
- Measured ratios: see `docs/qa-report.md`. Starlight's default blue is not used anywhere.
