# 0004 — Visual QA: screenshot-and-eyeball gate

Date: 2026-11-08

## Decision

Visual QA is a blocking gate, not a nice-to-have:

1. `scripts/qa-screenshots.mjs` walks `dist/` for every `index.html` (never a hand-maintained
   route list), maps files to URLs, adds `dist/404.html` once, and screenshots each route in
   both viewports (1440×900 desktop, 390×844 mobile) and both themes (light, dark) against a
   **production preview** (`pnpm preview`), never the dev server.
2. It fails on: console errors, page errors, HTTP ≥ 400, horizontal overflow, missing or
   duplicate `<h1>`, blank `<main>` (<200 chars), theme-application failure, broken images,
   a non-PNG logo (`logo-format`), and a logo rendered larger than its native pixels
   (`logo-upscale`, the pixelation guard). Failures are written to `.qa/failures.json`.
3. The 404 route is screenshotted but exempt from the `<h1>`/content-length checks.
4. `@axe-core/playwright` was **not** added: the project's own WCAG AA obligations are
   verified by measured contrast ratios (see `docs/qa-report.md`) and the structural checks
   above; adding the axe dependency was judged out of proportion for 14 static pages.

## Theme application

The script sets `localStorage['starlight-theme']` before navigation and verifies
`document.documentElement.dataset.theme`; it clicks the theme toggle as a fallback when
Starlight defers application.

## Results

Final run: 14 routes × 2 viewports × 2 themes = 56 screenshots, **QA PASSED**, zero
failures. The manual eyeball pass (homepage, tutorial, one guide, two references, one
explanation; both viewports; both themes for the homepage and tutorial) is recorded row by
row in `docs/qa-report.md`.

## Re-run rule

Any change to config, CSS, logo, favicon, theme or content invalidates the pass. This
happened twice during development (route-slug fix; accent AA fix) and the full
build → preview → qa cycle was re-run both times, with the affected screenshots re-inspected.
