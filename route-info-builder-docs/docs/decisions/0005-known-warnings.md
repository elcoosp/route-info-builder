# 0005 — Known warnings and documented exceptions

Date: 2026-11-08

The zero-warning gate (build, check, lint) passes. This file records the only exceptions,
each narrow and justified. Blanket suppression is not used.

## 1. Sitemap line in the build log (documented narrow `grep -v`)

Starlight 0.42.10 unconditionally injects `@astrojs/sitemap` unless it is already present
(`dist/index.js`: `if (!allIntegrations.find(({ name }) => name === "@astrojs/sitemap"))
integrations.push(starlightSitemap(...))`), and the integration prints exactly one warning
when no `site` URL is configured:

```
20:0x:xx [WARN] [@astrojs/sitemap] The Sitemap integration requires the `site` astro.config option. Skipping.
```

The route-info-builder repository declares no deploy URL (its workflows are `ci.yml` and
`release.yml`; no GitHub Pages deployment exists), and inventing a URL would violate the
accuracy principle. The line is therefore excluded — and only that exact string — in
`scripts/quality-gate.sh`, with the comment inline. Any other warning fails the gate.

## 2. `astro check` replaced by `tsc --noEmit` (environment defect)

`@astrojs/check@0.9.12` (latest) crashes on import in this environment:

```
TypeError: this.bridge.setLastError is not a function
    at NodeEnv.deleteEnv (.../node_modules/@emnapi/runtime/dist/emnapi.js:855:21)
```

Root cause: `@astrojs/check` → `@astrojs/language-server` → `@astrojs/astro2tsx` →
`@napi-rs/wasm-runtime@1.2.4` loads a wasm module built against an `@emnapi/runtime` API
newer than any published runtime (`2.0.0-beta.2` and `2.0.0-beta.3` both lack
`setLastError`). This is a third-party package defect, not project code. Because the
project has no `.astro` component files, `astro sync && tsc --noEmit` (TypeScript 6.0.3,
Astro `strict` tsconfig) covers the same TypeScript surface. See `0001-typescript-version.md`.

## 3. Starlight content-layer warnings eliminated at the source (for the record)

Two further warnings appeared during development and were fixed rather than suppressed:

- `The collection "i18n" does not exist or is empty.` — fixed by declaring the `i18n`
  collection with Starlight's `i18nLoader()`/`i18nSchema()` in `src/content.config.ts` and
  shipping `src/content/i18n/en.json` (no UI-string overrides).
- `Could not render /404 from route /[...slug] as it conflicts with higher priority route /404.`
  and, after removing the page, `Entry docs → 404 was not found.` — fixed by the documented
  0.42 combination: a `404.md` content page **plus** `disable404Route: true`, so exactly one
  route renders `/404`.

## Hints

`astro sync`, `tsc --noEmit` and `markdownlint-cli2` emit zero hints on the final build.
