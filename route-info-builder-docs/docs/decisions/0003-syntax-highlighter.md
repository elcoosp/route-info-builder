# 0003 — Syntax highlighter: Shiki via Expressive Code

Date: 2026-11-08

## Decision

Ship Starlight's default highlighter: **Shiki**, as integrated by Expressive Code.
No custom highlighter is added (no Rangi, no Prism, no highlight.js).

## Reason

Starlight's architecture builds code rendering on Expressive Code, whose supported
highlighter is Shiki (TextMate grammars, 180+ languages, build-time rendering with zero
client JavaScript, native dual-theme support). A documentation site with Rust, TOML,
TypeScript, TSX and bash samples is exactly the designated use case.

Rangi targets chat UIs and small bundles (~46 languages, regex lexing, no Expressive Code
integration). Adopting it here would mean maintaining an unsupported integration while
losing transformers, Twoslash and the dual-theme pipeline.

## Configuration

`expressiveCode: { themes: ['github-light', 'github-dark'] }` — a dual-theme pair wired to
Starlight's light/dark toggle. The `shiki.langs` list was removed after `tsc --noEmit`
flagged it: plain strings are not `LanguageInput` values, and every language used on the
site (js, ts, tsx, md, mdx, bash, json, yaml, css, html, rust, toml, ansi output) is in
Shiki's bundled grammar set, so a custom list had no concrete need.

## Authoring rules enforced

- Every fence declares a language (`ansi` for terminal output).
- Line highlighting was not needed on any page; the copy button ships with Expressive Code.
