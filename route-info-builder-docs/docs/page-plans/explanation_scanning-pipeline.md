---
slug: explanation/scanning-pipeline
type: explanation
audience: a developer who wants to reason about why routes are or are not detected and what the scanner can and cannot know
goal: understand the scan pipeline from file collection to handler metadata and its deliberate limits
scope:
  in: file collection rules and determinism, the routes() function lookup, statement-level walking of prefix/add/route/nest/merge, method-router chains, path canonicalization, handler metadata extraction (body/query/auth/return/errors), warnings
  out: operational fixes (see guides/troubleshoot-scanning), naming math (see explanation/naming-and-collisions), exhaustive config tables (see reference/configuration)
outline:
  - What the scanner is (build-time AST reader, not a compiler plugin)
  - Stage 1 — collect files (recursive, sorted, skips)
  - Stage 2 — find the routes function
  - Stage 3 — walk registration statements (Loco vs Axum, nest/merge, chains)
  - Stage 4 — canonicalize paths
  - Stage 5 — enrich with handler metadata
  - Deduplication and warnings
  - What the scanner deliberately does not know (handler module paths, type shapes)
source_refs:
  - src/parser.rs (scan_routes, walk_directory, deduplicate_routes, warn_on_inconsistent_parameters)
  - src/parser/routes.rs (extract, visit_method_call, HTTP_VERBS, prefix/nest/merge handling)
  - src/parser/handlers.rs (Visit-based handler extraction, json() conversion analysis)
  - src/utils/path.rs (param_name, canonical_segment, normalize_path, build_full_path)
  - docs/IMPROVEMENTS.md (design notes, verified maintainer record)
approved_by: self (autonomous pipeline)
---
