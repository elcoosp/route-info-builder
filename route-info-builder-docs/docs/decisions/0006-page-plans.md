# 0006 — Page plans and information architecture

Date: 2026-11-08

Every documentation page has a binding plan in `docs/page-plans/` written before the prose.
The four Diátaxis planning facts (type, audience, goal, scope.in/scope.out) and the
`source_refs` evidence trail are recorded per page. This file is the aggregate index.

## Aggregate

| # | Page (slug) | Type | Reader goal | Plan |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `/` (splash) | landing (not Diátaxis) | understand the tool, self-route into a quadrant | `index.md` |
| 2 | `tutorials/getting-started` | tutorial | zero → generated artifacts → one typed call per side | `tutorials_getting-started.md` |
| 3 | `guides/generate-from-cli` | how-to | produce every artifact from the CLI with flag overrides | `guides_generate-from-cli.md` |
| 4 | `guides/configure-naming` | how-to | shape variant/field names and resolve collisions | `guides_configure-naming.md` |
| 5 | `guides/wire-authentication` | how-to | make the generated client send bearer tokens | `guides_wire-authentication.md` |
| 6 | `guides/troubleshoot-scanning` | how-to | fix the four documented scanner failure modes | `guides_troubleshoot-scanning.md` |
| 7 | `reference/configuration` | reference | look up any config key's type/default/effect | `reference_configuration.md` |
| 8 | `reference/cli` | reference | look up sub-commands, flags, exit codes | `reference_cli.md` |
| 9 | `reference/api` | reference | look up functions, types, generators, errors | `reference_api.md` |
| 10 | `reference/route-info-json` | reference | look up the JSON route map schema | `reference_route-info-json.md` |
| 11 | `explanation/scanning-pipeline` | explanation | understand what the scanner sees and why | `explanation_scanning-pipeline.md` |
| 12 | `explanation/naming-and-collisions` | explanation | understand path → variant-name derivation | `explanation_naming-and-collisions.md` |
| 13 | `explanation/design-decisions` | explanation | understand recorded trade-offs and rejected alternatives | `explanation_design-decisions.md` |

## Coverage rules

- Every public feature has both a how-to and a reference entry: CLI (3 ↔ 8), naming (4 ↔ 7/12),
  auth (5 ↔ 7), scanning diagnostics (6 ↔ 11), route map (scan command in 3 ↔ 10), library
  API (2's build-script path ↔ 9).
- No page mixes quadrants; the homepage carries no `type` frontmatter.
- `/404` is a utility splash page (no Diátaxis type), rendered via the `404.md` content page
  with Starlight's injected 404 route disabled (see `0005-known-warnings.md` §3).

## Command verification note

The build environment has no Rust toolchain, so tutorial commands could not be executed
here. Verification evidence used instead: the crate's own CI-tested binary tests
(`tests/cli_binary.rs` — `generate`/`scan`/`init`/exit codes via `CARGO_BIN_EXE`),
integration tests for generated artifact shapes (`tests/integration.rs`), the crate's
`#[cfg(test)]` assertions on generated content (`export const client`, `pub enum Link`,
`"openapi": "3.1.0"`, `"handlerInfo"`), and the CLI flags/config keys read directly from
`src/cli.rs`/`src/configuration.rs`. This is recorded as the accuracy evidence trail for
`tutorials/getting-started` and `guides/generate-from-cli`.

## Revisions

- Original plans (13 files) written before any prose; no plan required substantive revision
  during writing. Two implementation-level corrections happened after the plans and are
  recorded in the other ADRs: the `shiki.langs` removal (0003) and the 404 route handling
  (0005).

## Plugins

Only `starlight-links-validator` is installed (broken internal links fail the build — a
release blocker per the quality gates). The other catalogued plugins are omitted for
concrete reasons: `starlight-typedoc` (the API is Rust, not TypeScript — the reference
pages are hand-written from source with `source_refs`), `starlight-blog`/`starlight-versions`
(no changelog section or versioned docs requested; the crate's CHANGELOG lives in the
repository), `starlight-openapi` (no OpenAPI spec describes this docs site), and
`starlight-image-zoom`/`starlight-heading-badges` (no images or badge-bearing headings in
the content). Search stays on Pagefind, the zero-config default.
