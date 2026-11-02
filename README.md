<div align="center">
  <img src="docs/logo.png" alt="Route Info Builder Logo" width="300"/>
  <p>
    <strong>Route Info Builder</strong><br/>
    A build-time route scanner for Axum-like frameworks: type-safe Rust link
    enums, a typed TypeScript client with TanStack Query hooks, an OpenAPI 3.1
    skeleton and a JSON route map — all from your existing controller files.
  </p>
  <p>
    <a href="https://github.com/elcoosp/route-info-builder/actions/workflows/ci.yml"><img src="https://github.com/elcoosp/route-info-builder/actions/workflows/ci.yml/badge.svg" alt="CI"/></a>
    <img src="https://img.shields.io/badge/MSRV-1.88%20%7C%202024-000000?style=flat-square&logo=rust" alt="MSRV"/>
    <img src="https://img.shields.io/crates/v/route-info-builder?style=flat-square" alt="Crates.io"/>
    <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License MIT"/>
    <img src="https://img.shields.io/badge/Target-TypeScript%20%2B%20React%20Query-3178C6?style=flat-square" alt="TypeScript"/>
  </p>
</div>

---

`route-info-builder` bridges the gap between your Rust backend and frontend by
scanning your Axum/Loco controllers and generating fully type-safe routing
enums and API clients. Eliminate hardcoded URLs and manual client boilerplate.

## Features

- **Automatic route scanning**: parses controller files recursively and
  deterministically (sorted order, `mod.rs` and dot-files skipped).
- **Two router styles**: Loco (`Routes::new().prefix(..).add(..)`) and plain
  Axum (`Router::new().route(..)`, `.nest(..)`, `.merge(..)`), including
  method-router chains such as `get(one).post(two)`.
- **Both path syntaxes**: Axum 0.8 `{param}` / `{*wildcard}` and legacy
  `:param` / `*wildcard` (legacy input is canonicalized).
- **Rust link enum**: `to_path()`, `method()` and `name()` on one generated
  enum, pretty-printed with `prettyplease`, optional serde derives.
- **TypeScript client + TanStack Query hooks**: typed path/query/body
  parameters, typed error unions per route, abort-signal plumbing.
- **OpenAPI 3.1 skeleton** and a **JSON route map** for tooling.
- **Typed errors** (`thiserror`) and `cargo:warning` diagnostics for
  duplicate routes and inconsistent parameter names.
- **CLI binary**: generate artifacts without a build script, or scaffold a
  configuration file.

## Installation

Add the crate to your `Cargo.toml` (or use the CLI):

```toml
[build-dependencies]
route-info-builder = "0.2"
```

The CLI is a default feature; turn it off for a lean library-only build:

```toml
[build-dependencies]
route-info-builder = { version = "0.2", default-features = false }
```

## Quick start

### 1. Configure the build script

```rust
// build.rs
use route_info_builder::{Config, RustConfig, TypeScriptConfig};

fn main() {
    let config = Config {
        controllers_path: "src/controllers".into(),
        rust: RustConfig {
            output_file: Some("src/generated/links.rs".into()),
            ..RustConfig::default()
        },
        typescript: TypeScriptConfig {
            generate_client: Some(true),
            output_path: Some("frontend/src/api".into()),
            ..TypeScriptConfig::default()
        },
        ..Config::default()
    };

    route_info_builder::generate_links(&config).expect("failed to generate route links");
}
```

Or generate the same artifacts from the command line:

```bash
cargo install route-info-builder
route-info-builder init              # writes a starter route-info.toml
route-info-builder generate          # builds every configured output
route-info-builder scan              # prints the JSON route map to stdout
```

### 2. Use the generated links in Rust

```rust
use crate::generated::links::Link;

// Type-safe route usage with parameters.
let user_link = Link::GetUser { user_id: "123".to_string() };

println!("Path: {}", user_link.to_path());   // "/api/users/123"
println!("Method: {}", user_link.method());  // "GET"
println!("Name: {}", user_link.name());      // "getUser" (matches the TS client key)

// In Axum router setup.
app.route(&Link::GetUsers.to_path(), axum::routing::get(get_users_handler));
```

### 3. Use the TypeScript client in the frontend

```typescript
import { useGetUsers, useCreateUser } from "./api/api";

function UserList() {
  const { data: users, isLoading } = useGetUsers();
  const createUser = useCreateUser();

  if (isLoading) return <div>Loading...</div>;

  return (
    <div>
      {users.map((user) => <div key={user.id}>{user.name}</div>)}
      <button onClick={() => createUser.mutate({ name: "New User" })}>
        Create User
      </button>
    </div>
  );
}
```

## What the scanner understands

```rust
// Loco style
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/users")
        .add("/", get(list_users))                    // GET  /api/users
        .add("/{user_id}", get(get_user).put(update_user))
        .add("/", post(create_user))
}

// Axum style with nesting and legacy parameters
pub fn routes() -> Router {
    Router::new()
        .route("/api/posts", get(list_posts))
        .route("/legacy/:legacy_id", get(get_legacy)) // canonicalized to {legacy_id}
        .nest("/api/admin", Router::new().route("/stats", get(admin_stats)))
        .merge(other_routes())
}
```

Handler metadata is extracted from the same file: `Json<T>`,
`JsonValidate<T>`, `JsonValidateWithMessage<T>` body extractors, `Query<T>`
extractors, an `auth: JWT` extractor (flags the route as authenticated) and
`format::json(...)` return conversions (including `Vec::<T>::new()` →
`Array<T>`). `Err(BadRequest::EmailAlreadyExists.into())` returns become
per-route TypeScript error unions.

## Configuration

Everything is optional except `controllers_path`. A full `route-info.toml`:

```toml
controllers_path = "src/controllers"
controllers_fn   = "routes"          # function scanned inside controller files

[naming]
include_method_in_names = true       # get_users -> GetUsers variants
path_prefix_to_remove   = "/api"     # affects names only, not URLs
variant_case            = "pascal"   # camel | pascal | snake | kebab | title | lower | upper
field_case              = "snake"
word_separators         = "-."       # extra word separators
preserve_numbers        = false      # glue digits to the previous word (v1 not v_1)
variant_prefix          = ""
variant_suffix          = ""
collision_strategy      = "skip"     # skip | append-method | number

[rust]
output_file = "src/generated/links.rs"
enum_name   = "Link"
derive_serde = false

[typescript]
generate_client = true
output_path     = "frontend/src/api"
client_file     = "client.ts"
hooks_file      = "api.ts"
bindings_path   = "../../../bindings"  # where DTO types live

[typescript.auth]                      # omit for an anonymous client
token_key_import = "TOKEN_KEY"
import_from      = "@/hooks/use-auth"

[openapi]
output_path = "openapi.json"
title       = "My API"
version     = "0.1.0"

[route_map]
output_path = "routes.json"
```

## Public API

| Function | Purpose |
| :--- | :--- |
| `generate_links(&Config)` | Scan once; return the Rust enum and write every configured output. |
| `scan_routes(&Config)` / `scan_controllers_folder(&Config)` | Return the scanned `Vec<RouteInfo>`. |
| `generate_ts_client(&Config)` / `generate_ts_hooks(&Config)` | Single TypeScript artifacts. |
| `generate_openapi_json(&Config)` | OpenAPI 3.1 skeleton as a JSON string. |
| `generate_route_map_json(&Config)` | JSON route map as a string. |
| `Config::from_toml_file` / `from_toml_str` / `load` | Configuration loading. |

Generators implement `CodeGenerator` and can be driven directly:
`RustLinksGenerator`, `TypeScriptClientGenerator`, `TypeScriptHooksGenerator`
and `OpenApiGenerator`.

## Generated output examples

### Rust enum

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Link {
    GetApiUsers,
    GetApiUsersUserId { user_id: String },
    PostApiUsers,
}

impl Link {
    pub fn to_path(&self) -> String { /* "/api/users/{0}", user_id */ }
    pub fn method(&self) -> &'static str { /* "GET", "POST", ... */ }
    pub fn name(&self) -> &'static str { /* "getApiUsers", ... */ }
}
```

### TypeScript client (excerpt)

```typescript
export interface GetApiUsersUserIdParams {
  userId: string;
}

export const client = {
  getApiUsersUserId: async (params: GetApiUsersUserIdParams, config?: { signal?: AbortSignal }): Promise<UserResponse> => {
    const url = `/api/users/${params.userId}`;
    return apiClient.get<UserResponse, ApiError>(url, { requiresAuth: true, signal: config?.signal });
  },
};
```

## Troubleshooting

> [!WARNING]
> **Common issues**
> - **Routes not found**: the scanner looks for a function named `routes`
>   (override with `controllers_fn`) inside `*.rs` files under
>   `controllers_path`. `mod.rs` files and dot-files are skipped, and routes
>   built inside local variables must be registered through a statement
>   chain (`let router = Router::new(); router.route(...);`) to be seen.
> - **Duplicate variants**: with the default `skip` strategy duplicates are
>   dropped with a `cargo:warning`; switch to `append-method` or `number` to
>   keep both routes.
> - **Inconsistent parameter names**: routes sharing the same path shape but
>   different parameter names (e.g. `/items/{id}` and `/items/{item_id}`)
>   produce a warning; align the names.
> - **Build errors**: all controller files must contain valid Rust — the
>   scanner parses them as ASTs and reports `Parse` errors with file context.

## Development

```bash
cargo fmt --check          # formatting
cargo clippy --all-targets # pedantic, deny-by-default lints
cargo test --all-features  # 150+ tests, doc tests included
cargo llvm-cov --all-features --fail-under-lines 100
```

CI runs all of the above on stable plus the 1.88 MSRV; releases are cut with
[git-cliff](https://git-cliff.org) changelogs (see `.github/workflows`).

## License

MIT — see [LICENSE-MIT](LICENSE-MIT).
