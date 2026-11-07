---
slug: reference/cli
type: reference
audience: operators and developers running the route-info-builder binary
goal: look up any sub-command, flag, default lookup file and exit code
scope:
  in: generate/scan/init sub-commands, GenerateArgs flags (-c/--config, --controllers, --rust-output, --ts-dir, --openapi, --route-map), init [path] --force, exit codes, default route-info.toml lookup, the cli feature gate
  out: library equivalents (see reference/api), config key semantics (see reference/configuration), teaching material (see tutorials/getting-started)
outline:
  - Overview (binary name, feature gate, version)
  - Commands table
  - generate flags table + per-flag detail (override mapping)
  - scan output description
  - init behavior and --force
  - Exit codes table
  - route-info.toml auto-detection
source_refs:
  - src/cli.rs (Cli, Command, GenerateArgs, run/dispatch exit codes, DEFAULT_CONFIG_FILE)
  - Cargo.toml ([features] cli, [[bin]])
  - tests/cli_binary.rs (exit codes verified)
approved_by: self (autonomous pipeline)
---
