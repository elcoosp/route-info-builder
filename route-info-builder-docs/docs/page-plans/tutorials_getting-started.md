---
slug: tutorials/getting-started
type: tutorial
audience: a Rust backend developer new to route-info-builder who also maintains a TypeScript frontend
goal: go from zero to generated Rust link enum + TypeScript client + TanStack Query hooks and make one typed call from each side
scope:
  in: build.rs integration path (crate as build-dependency), one Loco-style and one Axum-style controller, `cargo build` run, importing `Link` in Rust, importing hooks in TSX
  out: CLI usage (see guides/generate-from-cli), naming configuration (see guides/configure-naming), auth plumbing (see guides/wire-authentication), why the scanner works this way (see explanation/scanning-pipeline)
outline:
  - One-sentence purpose statement
  - Prerequisites (Rust 1.88+, Node + pnpm + TanStack Query for the frontend half)
  - Step 1 — create the Axum project and controller file
  - Step 2 — add route-info-builder as a build-dependency
  - Step 3 — write build.rs with a minimal Config
  - Step 4 — run cargo build and confirm the generated files (you should see…)
  - Step 5 — use the Link enum in Rust
  - Step 6 — install TanStack Query and use a generated hook in React
  - Next steps (LinkCard to guides)
source_refs:
  - README.md (quick start, verified against source)
  - src/lib.rs (generate_links contract: writes outputs, returns enum code)
  - src/lib.rs doc example (end-to-end fixture)
  - src/cli.rs (STARTER_CONFIG defaults)
  - src/generators/typescript/hooks.rs (useQuery for GET / useMutation otherwise)
  - tests/cli_binary.rs (CLI behavior verified by the crate's own tests)
  - tests/integration.rs (generated artifact shapes)
approved_by: self (autonomous pipeline)
---
