---
slug: reference/api
type: reference
audience: developers driving route-info-builder as a library from build.rs or other Rust code
goal: look up every public function, type, trait and generator with its signature, semantics and error behavior
scope:
  in: the seven public functions, the Config family, RouteInfo/HandlerInfo/ReturnType, Error variants, CodeGenerator trait and the four generator structs, crate feature gate
  out: TOML key semantics (see reference/configuration), CLI surface (see reference/cli), design rationale (see explanation/design-decisions)
outline:
  - Overview (crate layout, feature gate)
  - Functions table + per-function detail (writes-to-disk side effects)
  - Configuration types table
  - Scanned-data types (RouteInfo, HandlerInfo, ReturnType)
  - CodeGenerator trait + generator structs table
  - Error enum table (variant, meaning, source chain)
source_refs:
  - src/lib.rs (exports, generate_links side effects, write_output)
  - src/parser.rs (RouteInfo, HandlerInfo, scan signatures)
  - src/parser/handlers.rs (ReturnType fields)
  - src/generators.rs (CodeGenerator trait)
  - src/generators/rust.rs, typescript/client.rs, typescript/hooks.rs, openapi.rs (generator structs)
  - src/error.rs (Error variants)
approved_by: self (autonomous pipeline)
---
