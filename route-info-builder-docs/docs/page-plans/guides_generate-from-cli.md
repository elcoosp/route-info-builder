---
slug: guides/generate-from-cli
type: how-to
audience: a developer who wants artifacts without wiring a build script
goal: install the CLI and produce every artifact from the command line, overriding outputs with flags
scope:
  in: cargo install, init (and --force), generate, scan, the six shared flags, exit codes 0/1/2
  out: library API (see reference/api), config key semantics (see reference/configuration), teaching the scanner model (see explanation/scanning-pipeline)
outline:
  - Task statement
  - Preconditions (cargo on PATH, controllers folder exists)
  - Install and scaffold route-info.toml
  - Generate artifacts; read the printed summary
  - Override outputs per invocation with flags
  - Inspect routes with scan
  - Exit codes and failure modes
  - Next steps (LinkCard to reference/cli)
source_refs:
  - src/cli.rs (Command enum, GenerateArgs, load_config overrides, exit codes)
  - src/main.rs (binary entry)
  - tests/cli_binary.rs (verified through CARGO_BIN_EXE tests)
approved_by: self (autonomous pipeline)
---
