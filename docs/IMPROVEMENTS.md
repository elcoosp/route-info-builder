# Improvement brainstorm — route-info-builder 0.2.0

This document records the brainstorm performed while upgrading the crate and
which items were implemented. It doubles as the design rationale for the
0.2.0 release.

## 1. Bugs found and fixed

| # | Bug | Fix |
| --- | --- | --- |
| 1 | `ts_quote` was a local path dependency (`../type_reflect/ts_quote`) that broke the build for any external consumer; the published `ts_quote 0.5.0` itself fails to compile on crates.io (two incompatible `deno_ast` versions in its tree) | Dropped the dependency; TS emission is now plain, fully tested string building |
| 2 | `deno_ast` was declared but never used, while `format_ts_code` was a no-op — a huge (swc-based) compile-time tax | Removed; generated code is emitted pre-formatted |
| 3 | `generate_links` never created output directories, so first runs crashed when `frontend/src/api` did not exist | All writes create parent directories (`Error::CreateDir`) |
| 4 | `path_prefix_to_remove` used a raw `starts_with` check: prefix `/user` corrupted the name of a `/users` route | Prefix stripping is segment-aware |
| 5 | `include_method_in_names` defaulted to `true` in code while the README said `false` | Documented, single source of truth via `NamingConfig::include_method()` |
| 6 | `preserve_numbers` config existed but was dead code | Implemented via `convert_case` digit-boundary removal (`preserve_numbers = false`) |
| 7 | Route names that collide with Rust keywords (`/match`) or empty parameter names (`{}`) panicked in `Ident::new` | `to_valid_ident` emits raw identifiers (`r#match`) or safe fallbacks; degenerate params are treated as literals |
| 8 | Scanning order followed `read_dir` order → non-deterministic output | Files collected recursively and sorted; all generators iterate deterministic `BTree` collections |
| 9 | Duplicate variant names were silently skipped | Configurable `collision_strategy` (`skip` with warning, `append-method`, `number`) |
| 10 | Every generated client hard-coded `import { TOKEN_KEY } from "@/hooks/use-auth"` and `../../../bindings` import paths (with a `FIXME`) | Authentication plumbing and the bindings path are configurable; the client works without auth |
| 11 | The hand-rolled return-type recursion double-visited tails and missed closures/`try` blocks | Replaced with `syn`'s `visit` infrastructure; error collection ignores plain variables (a false "error type" in the original) |
| 12 | `Box<dyn Error>` errors lost file/method context | Typed `Error` enum (`thiserror`) carrying paths and sources |
| 13 | The README documented a `Config` API that did not exist | README rewritten against the real API, doc-tested examples added |
| 14 | Path-shape validation (added) compared parameter names across different positions, producing false positives | Positions are compared independently |

## 2. Performance

- Removed `deno_ast` + `ts_quote`: drops the entire swc dependency tree from
  the build (the dominant compile cost of 0.1.0).
- Generated Rust is pretty-printed once with `prettyplease` instead of
  requiring a downstream `cargo fmt` pass.
- Single-pass separator collapsing in name generation (replaces the
  `while contains("__") { replace }` loop).
- `BTreeMap`/`BTreeSet` everywhere output is affected → deterministic output
  with no sorting passes at the end.
- Separator/brace filtering is a single `chars().map()` pass.

## 3. Modularity, DRY, KISS

- The 16 near-identical TypeScript branches (4 GET + 4 non-GET shapes in the
  client and again in the hooks) collapsed into one parameter-descriptor code
  path per file: ~500 fewer lines and only one place to extend.
- Handler analysis split into two single-purpose `syn::visit::Visit`
  implementations (return-type conversion vs error collection) instead of one
  interleaved state machine.
- Naming rules live in one module (`configuration::naming`) including the
  `VariantNamer` collision strategies.
- Path syntax normalization (`{p}`, `:p`, `{*p}`, `*p`) happens once in
  `utils::path::canonical_segment`; every generator consumes normalized paths.
- The generated `format!` template uses numbered placeholders so repeated
  parameters cannot desynchronize fields and arguments.
- Crate layout moved to the non-`mod.rs` module style, and `Config` grew
  `deny_unknown_fields` so TOML typos fail loudly.

## 4. New complementary features

1. **Typed error enum** with file paths and `#[source]` chains.
2. **TOML configuration files**: `Config::from_toml_str`, `from_toml_file`,
   and `Config::load` (auto-detects `route-info.toml`).
3. **Recursive controller scanning** (subdirectories, deterministic order,
   `mod.rs`/dot-file skipping, non-UTF8 name tolerance).
4. **Axum-native router support**: `route`, `nest` (with prefix stacking),
   `merge`; all HTTP verbs including `head`/`options`/`trace`, verb chains
   (`get(a).post(b)` → one route per verb), and a generic
   `apiClient.request` fallback for unusual verbs.
5. **Axum 0.8 + legacy path syntax** with canonicalization, wildcards
   included.
6. **Collision strategies** for variant names (skip / append-method / number).
7. **Route validation warnings** for duplicate `(method, path)` pairs and
   mixed parameter names at the same path position.
8. **Configurable generated enum**: custom name, optional serde derives,
   `Eq`/`Hash`, `name()` for query-key parity with the TS client.
9. **JSON route map** (`route_map.output_path` or `generate_route_map_json`)
   — serializable `RouteInfo` with camelCase keys.
10. **OpenAPI 3.1 skeleton** (`openapi.output_path` or
    `generate_openapi_json`) with path/query parameters, request/response
    schema refs, per-route error unions and bearer security.
11. **CLI binary** (`route-info-builder generate|scan|init`) behind a
    `cli` feature so library consumers do not pay for `clap`.
12. **Configurable TypeScript output**: file names, bindings path, optional
    auth import (or a fully anonymous client).

## 5. Testing and CI

- 150+ tests: unit tests per module, integration tests over a fixture
  controller tree (Loco + Axum + legacy syntax), CLI tests through the public
  `run` function, binary tests via `CARGO_BIN_EXE`, and doc tests.
- **100% line coverage** enforced by `cargo llvm-cov --fail-under-lines 100`.
- GitHub Actions CI (fmt, pedantic clippy, test matrix on Linux/macOS/Windows
  plus the 1.88 MSRV, coverage gate, packaging/doc dry run, informational
  semver-checks) and a release workflow with git-cliff changelogs, a binary
  upload and optional crates.io publishing.
- Dependabot keeps Cargo and Actions dependencies fresh.

## 6. Considered but rejected

- **`walkdir`**: recursion is ~20 lines of std code; not worth the dependency.
- **`insta` snapshot tests**: exact-string assertions keep the test surface
  explicit and avoid a snapshot-update workflow for a small crate.
- **Zod/serde-JSON schema generation**: the scanner only knows type names,
  not shapes, so generated schemas would be fiction; the OpenAPI skeleton
  documents the same limitation honestly.
- **A generated `Router` builder**: registering handlers requires their
  module paths, which the scanner deliberately does not guess.
- **Watch mode**: out of scope for a build-script tool; `cargo watch` covers
  it.
