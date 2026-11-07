# 0001 — Pin TypeScript to 6.x

Date: 2026-11-08

## Decision

`typescript` is pinned to exactly `6.0.3` in `devDependencies` (the latest 6.x release).
The registry's latest TypeScript is `7.0.2`, and it is intentionally not used.

## Reason

`astro check` refuses TypeScript 7:

```
astro check does not currently support TypeScript 7.0
```

The skill baseline requires `astro check` to pass with 0 errors and 0 warnings, so the
6.x line is the supported combination.

## Environment note

In this build environment `astro check` could not be executed at all:
`@astrojs/check@0.9.12` crashes on import with

```
TypeError: this.bridge.setLastError is not a function
    at NodeEnv.deleteMe (.../node_modules/@emnapi/runtime/dist/emnapi.js:855:21)
```

`@astrojs/check` → `@astrojs/language-server` → `@astrojs/astro2tsx` → `@napi-rs/wasm-runtime@1.2.4`
loads a wasm binding built against an `@emnapi/runtime` API newer than any published runtime
(beta.2 and beta.3 both lack `setLastError`). Pinning overrides cannot fix a wasm/JS API
mismatch inside the published package.

**Equivalent gate adopted:** the `check` script runs `astro sync && tsc --noEmit` with
TypeScript 6.0.3 and Astro's `strict` tsconfig. The project contains no `.astro` component
files, so `tsc --noEmit` covers the entire TypeScript surface that `astro check` would
(`src/content.config.ts`, `src/env.d.ts`, generated `.astro/types.d.ts`). It caught and
rejected a real config type error during development. See `0005-known-warnings.md`.
