# AGENTS.md — Route Info Builder documentation site

Instructions for AI agents and humans maintaining this Astro Starlight documentation site
for the [`route-info-builder`](https://github.com/elcoosp/route-info-builder) crate.

## What this is

A Diátaxis-organized Starlight documentation site (Astro 7.4.x, `@astrojs/starlight`
0.42.x, pnpm). Content lives in `src/content/docs/`:

```
src/content/docs/
├── index.mdx              # splash homepage (mandatory, not a Diátaxis page)
├── 404.md                 # utility 404 (splash template)
├── tutorials/             # learning-oriented, one path, no alternatives
├── guides/                # task-oriented how-tos, assume competence
├── reference/             # information-oriented, austere, complete
└── explanation/           # understanding-oriented, discursive
```

## Non-negotiable rules

1. **One Diátaxis type per page.** Never mix tutorial steps into a reference, or teaching
   into a how-to. Each page's frontmatter carries `type: tutorial | how-to | reference |
   explanation`; the homepage carries none.
2. **Plan before prose.** Every new page gets a plan in `docs/page-plans/<slug>.md` with
   `type`, `audience`, `goal`, `scope.in`, `scope.out`, `outline` and `source_refs`
   before drafting. `source_refs` is the accuracy evidence trail; a reference page without
   it is invalid.
3. **Accuracy over plausibility.** Every command, flag, default and type must be traceable
   to the crate source (`src/` in the route-info-builder repository) or its tests. When the
   README and the code disagree, the code wins — and the discrepancy is recorded in
   `docs/decisions/`.
4. **Writing voice.** Second person, present tense, active voice, imperative for
   instructions. Never "simply", "just", "easy", "obviously".
5. **pnpm only.** No npm/yarn lockfiles. `<Tabs>` for package-manager alternatives, pnpm first.
6. **TypeScript stays on 6.x** (`astro check` does not support TypeScript 7). The `check`
   script is `astro sync && tsc --noEmit`; see `docs/decisions/0001-typescript-version.md`.
7. **The logo is the project's PNG**, byte-identical, never re-encoded lossily, never
   upscaled. Dark-mode legibility comes from the CSS backdrop in
   `src/styles/custom.css`, not from editing pixels. The favicon is `public/favicon.png`.
8. **Syntax highlighting is Shiki** via Expressive Code (`github-light`/`github-dark`).
   Do not introduce another highlighter.
9. **The accent color is sampled from the logo** (see `docs/decisions/0002-logo-format.md`).
   Contrast must keep WCAG AA in both themes.

## Commands

```bash
pnpm install            # after cloning or changing deps
pnpm dev                # local dev server
pnpm build              # production build (runs the llms.txt prebuild)
pnpm check              # astro sync + tsc --noEmit (0 errors required)
pnpm lint               # markdownlint over src/content/docs (0 issues required)
pnpm qa                 # Playwright screenshots, all routes × 2 viewports × 2 themes
bash scripts/quality-gate.sh   # build + check + lint with the zero-warning gate
```

QA requires a production preview: `pnpm build && pnpm preview --port 4321 &` then `pnpm qa`.
`.qa/` is never committed or shipped.

## Zero-warning policy

Any warning line in build/check/lint output is a release blocker. The only exceptions are
recorded in `docs/decisions/0005-known-warnings.md` with a narrow, commented filter in
`scripts/quality-gate.sh`. Any change to config, CSS, logo, favicon, theme or content
invalidates the QA pass — re-run `bash scripts/quality-gate.sh && pnpm qa` and re-inspect
the affected screenshots before shipping.

## AI discoverability

`scripts/build-llms-txt.mjs` (the `prebuild` hook) generates `public/llms.txt` and
`public/llms-full.txt` from the content tree. New pages are picked up automatically; keep
frontmatter `title`/`description` well-formed.
