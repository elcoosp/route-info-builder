# Visual QA report — Route Info Builder documentation site

Build: production (`pnpm build`), served with `pnpm preview --port 4321`, probed with
`node scripts/qa-screenshots.mjs` (Playwright Chromium, `reducedMotion: reduce`).

Final run: **14 routes × 2 viewports (1440×900, 390×844) × 2 themes = 56 screenshots — QA PASSED.**
The automated gate fails on console errors, page errors, HTTP ≥ 400, horizontal overflow,
missing/duplicate `<h1>`, blank `<main>`, theme-application failure, broken images, a non-PNG
logo, and a logo rendered larger than its native pixels. Zero failures were recorded
(`.qa/failures.json` is an empty array).

## Per-route eyeball results

Legend: ✓ = inspected, no defect. "—" = not inspected this pass (covered by the automated gate).

| Route | Desktop light | Desktop dark | Mobile light | Mobile dark | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `/` (splash homepage) | ✓ | ✓ | ✓ | ✓ | Splash hero + 4 `<LinkCard>`s; logo sharp at header and hero size; amber accent on button/title; dark mode renders the logo on a light rounded backdrop so the dark-gray links stay visible |
| `/tutorials/getting-started/` | ✓ | ✓ | — | ✓ | `<Steps>` numbered 1–7, `<Tabs>` with pnpm first, `<FileTree>` renders as a bordered tree, fenced code highlighted with multi-color tokens, `<Aside>` note/tip icons correct, TOC highlights current heading, pagination correct |
| `/guides/generate-from-cli/` | — | — | — | — | Same layout family as the inspected guide below; automated gate green |
| `/guides/configure-naming/` | — | — | — | — | Same layout family; automated gate green |
| `/guides/wire-authentication/` | ✓ | — | — | — | toml/rust/ts highlighting, amber Caution and blue Note asides, prev/next pagination correct |
| `/guides/troubleshoot-scanning/` | — | — | — | — | Same layout family; automated gate green |
| `/reference/configuration/` | ✓ | — | — | — | Seven tables fit without clipped columns; active sidebar item amber; caution aside styled |
| `/reference/cli/` | — | — | — | — | Same layout family; automated gate green |
| `/reference/api/` | — | — | — | — | Same layout family; automated gate green |
| `/reference/route-info-json/` | — | ✓ (mobile light inspected) | ✓ | — | Tables reflow and stay readable at 390 px; JSON example highlighted |
| `/explanation/scanning-pipeline/` | — | — | — | — | Same layout family; automated gate green |
| `/explanation/naming-and-collisions/` | — | — | — | — | Same layout family; automated gate green |
| `/explanation/design-decisions/` | ✓ | — | — | — | Discursive register, note aside, LinkCards, TOC correct |
| `/404.html` | — | — | — | — | Screenshot captured once per theme/viewport; excluded from content checks by design |

## Checklist (skill §11.5)

- [x] Homepage is a splash landing page — hero + `<CardGrid>`-equivalent `<LinkCard>` grid, no redirect, no docs-only layout.
- [x] Logo is the project's PNG (byte-identical copy, 1264×1264), sharp at both header and hero render sizes; no upscale (automated guard), no halo, no lossy artifacts.
- [x] Favicon resolves — `dist/favicon.png` exists and is referenced; no 404 (automated HTTP guard).
- [x] Brand accent (sampled logo amber) appears on the primary button, sidebar active state, and focus surfaces; it is not Starlight blue.
- [x] Code blocks are syntax-highlighted (Shiki/Expressive Code, `github-light`/`github-dark`), have the copy button, and scroll instead of overflowing.
- [x] Tables fit at both viewports; no clipped columns observed.
- [x] `<Aside>`, `<LinkCard>`, `<Tabs>`, `<Steps>`, `<FileTree>` render with correct icons, spacing, borders.
- [x] Sidebar active state correct; groups expand; nothing duplicated.
- [x] On-page TOC present on long pages and highlighting the current heading.
- [x] Pagination prev/next point to the correct adjacent pages (verified on tutorial, guide, reference, explanation pages).
- [x] Mobile: header collapses to the menu/search buttons; no clipping or overlap; content reflows.
- [x] Dark mode: no pure-white panels, no invisible text; code backgrounds distinct from page background; logo legible via the scoped CSS backdrop.
- [x] Typography: no overlap, no orphaned headings; nothing hidden under the fixed header.
- [x] Search: Pagefind modal opens (header search button), query "collision" returns sensible titles and excerpts ("Naming and collisions", "Configure generated names", "Resolve name collisions", …) — probe in `scripts/qa-search-probe.mjs`.
- [x] Type fidelity: tutorial contains no reference tables; references contain no steps; guides assume competence; explanations instruct nothing.

## Measured contrast (WCAG AA = 4.5:1 body text, 3:1 large text / UI)

| Surface | Ratio | Verdict |
| :--- | :--- | :--- |
| Light link text `--sl-color-accent-high` hsl(24 65% 28%) on white | 8.49:1 | AA pass |
| Light accent surface `--sl-color-accent` hsl(30 70% 37%) with white text (buttons, active sidebar) | 5.08:1 | AA pass |
| Dark link text `--sl-color-accent-high` hsl(34 85% 78%) on dark background | 11.66:1 | AA pass |
| Dark accent surface hsl(32 75% 58%) on dark background | 7.39:1 | AA pass |
| Dark button: dark text on accent hsl(32 75% 58%) | 7.39:1 | AA pass |

The sampled logo orange #bb6d21 (hsl(30 70% 43%)) measured 3.94:1 with white text — below AA for
body-size text — so the light-mode accent is deepened to hsl(30 70% 37%) (#a05e1c), same hue.
See `docs/decisions/0002-logo-format.md`.

## Re-run rule

The last actions before packaging were `bash scripts/quality-gate.sh` (zero-warning build/check/lint)
followed by `pnpm qa` (56/56 passing) and a re-inspection of the homepage screenshot after the final
accent change. The shipped state is the QA-passed state.
