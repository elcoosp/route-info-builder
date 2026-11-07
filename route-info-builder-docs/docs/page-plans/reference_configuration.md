---
slug: reference/configuration
type: reference
audience: developers writing a route-info.toml or building a Config struct in code
goal: look up any configuration key's type, default, effect and the file it is read from
scope:
  in: every key of Config, NamingConfig, RustConfig, TypeScriptConfig, AuthConfig, OpenApiConfig, RouteMapConfig; CollisionStrategy values; Config::load lookup order; deny_unknown_fields behavior
  out: step-by-step setup (see tutorials/getting-started), naming strategy rationale (see explanation/naming-and-collisions), CLI flag mapping (see reference/cli)
outline:
  - Overview (two ways to configure, load order)
  - Top-level keys table
  - [naming] keys table + per-key detail
  - [rust] keys table + per-key detail
  - [typescript] keys table + per-key detail (incl. [typescript.auth])
  - [openapi] and [route_map] keys tables
  - Unknown-key rejection and error message
source_refs:
  - src/configuration.rs (all structs, serde defaults, deny_unknown_fields, accessor methods)
  - src/configuration/naming.rs (naming semantics)
  - src/cli.rs (STARTER_CONFIG as the init template)
  - tests/cli_binary.rs (STARTER_CONFIG asserted)
approved_by: self (autonomous pipeline)
---
