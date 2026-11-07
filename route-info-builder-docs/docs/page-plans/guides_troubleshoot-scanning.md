---
slug: guides/troubleshoot-scanning
type: how-to
audience: a developer whose scan produced no routes, dropped routes, or failed the build
goal: diagnose and fix the four documented failure modes of the scanner
scope:
  in: routes not found (controllers_fn, mod.rs/dot-file skipping, statement-chain requirement), duplicate (method, path) routes and collision_strategy, inconsistent parameter names, Parse errors on invalid Rust
  out: how the AST walk works internally (see explanation/scanning-pipeline), error enum reference (see reference/api)
outline:
  - Task statement
  - "Routes not found: function name, skipped files, statement chains"
  - "Routes missing: duplicate (method, path) pairs are dropped with a cargo:warning"
  - "Warnings: inconsistent parameter names at the same path shape"
  - "Build fails: Parse errors with file context"
  - Next steps (LinkCard to explanation/scanning-pipeline)
source_refs:
  - src/parser.rs (scan_routes, collect_controller_files, deduplicate_routes, warn_on_inconsistent_parameters)
  - src/parser/routes.rs (extract: statements visited)
  - src/error.rs (Error::Parse with path context)
  - README.md (troubleshooting section)
approved_by: self (autonomous pipeline)
---
