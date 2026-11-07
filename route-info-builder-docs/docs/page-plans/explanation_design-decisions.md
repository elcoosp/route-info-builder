---
slug: explanation/design-decisions
type: explanation
audience: a developer evaluating route-info-builder or wondering why it looks the way it does
goal: understand the recorded decisions and trade-offs behind the 0.2.0 design, including rejected alternatives
scope:
  in: decisions recorded by the maintainers in docs/IMPROVEMENTS.md — dropped dependencies, deterministic output, skeleton honesty of the OpenAPI document, collision strategies, the rejected ideas list
  out: pipeline mechanics (see explanation/scanning-pipeline), API lookup (see reference/api); no claims about maintainer intent beyond what docs/IMPROVEMENTS.md records
outline:
  - Context (0.1.0 → 0.2.0 rework recorded by maintainers)
  - Zero heavy dependencies (ts_quote/deno_ast removal and why)
  - Determinism as a contract (sorted files, BTree collections, prettyplease)
  - The OpenAPI document is a skeleton by design
  - Errors with context (typed enum, file paths, source chains)
  - Considered and rejected (walkdir, snapshot tests, Zod/schema generation, router builder, watch mode)
source_refs:
  - docs/IMPROVEMENTS.md (sections 1, 2, 3, 4, 6 — the maintainer's own record)
  - src/error.rs (typed errors)
  - src/generators/openapi.rs (skeleton doc comment)
  - Cargo.toml (dependency list as evidence)
approved_by: self (autonomous pipeline)
---
