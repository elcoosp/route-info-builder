<div align="center">
  <img src="docs/logo.png" alt="Route Info Builder Logo" width="300"/>
  <p>
    <strong>Route Info Builder</strong><br/>
    A Rust procedural macro library for generating type-safe route information and TypeScript clients from Axum web framework routes.
  </p>
  <p>
    <img src="https://img.shields.io/badge/Rust-1.94%20%7C%202024-000000?style=flat-square&logo=rust" alt="Rust"/>
    <img src="https://img.shields.io/crates/v/route-info-builder?style=flat-square" alt="Crates.io"/>
    <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License MIT"/>
    <img src="https://img.shields.io/badge/Framework-Axum-000000?style=flat-square" alt="Axum"/>
    <img src="https://img.shields.io/badge/Target-TypeScript%20%2B%20React%20Query-3178C6?style=flat-square" alt="TypeScript"/>
  </p>
</div>

---

`route-info-builder` bridges the gap between your Rust backend and frontend by automatically scanning your Axum controllers and generating fully type-safe routing enums and API clients. Eliminate hardcoded URLs and manual client boilerplate.

## Features

- **Automatic Route Scanning**: Parses Axum controller files to extract route metadata.
- **Rust Enum Generation**: Produces type-safe Rust enums for all application routes with parameter validation.
- **TypeScript Client**: Generates a fully-typed TypeScript client with pre-built React Query hooks.
- **Compile-Time Safety**: Catch routing errors and mismatched parameters at compile time.
- **Highly Configurable**: Customize naming conventions, path prefixes, and output formats to match your project's style.

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
route-info-builder = "0.1"
```

## Quick Start

### 1. Configure the Build Script

Create a `build.rs` file in your project root to define the scanning and generation parameters:

```rust
use route_info_builder::Config;
use std::path::PathBuf;

fn main() {
    let config = Config {
        controllers_path: PathBuf::from("src/controllers"),
        output_file: Some("src/generated/links.rs".to_string()),
        generate_typescript_client: Some(true),
        typescript_client_output: Some(PathBuf::from("frontend/src/api/client.ts")),
        ..Default::default()
    };

    route_info_builder::generate_links(&config).expect("Failed to generate route links");
}
```

### 2. Use Generated Links in Rust

Import the generated enum to ensure type-safe route construction and Axum integration:

```rust
use crate::generated::links::Link;

// Type-safe route usage with parameters
let user_link = Link::GetUser { user_id: "123".to_string() };

println!("Path: {}", user_link.to_path());   // "/api/users/123"
println!("Method: {}", user_link.method());  // "GET"

// In Axum router setup
app.route(&Link::GetUsers.to_path(), get(get_users_handler))
   .route(&user_link.to_path(), get(get_user_handler));
```

### 3. Use the TypeScript Client in Frontend

Import the generated hooks directly into your React components:

```typescript
import { useGetUsers, useCreateUser } from './api/client';

function UserList() {
  const { data: users, isLoading } = useGetUsers();
  const createUser = useCreateUser();

  if (isLoading) return <div>Loading...</div>;

  return (
    <div>
      {users.map(user => <div key={user.id}>{user.name}</div>)}
      <button onClick={() => createUser.mutate({ name: 'New User' })}>
        Create User
      </button>
    </div>
  );
}
```

## Configuration

The `Config` struct provides extensive control over the generation process:

| Option | Type | Description | Default |
| :--- | :--- | :--- | :--- |
| `controllers_path` | `PathBuf` | **Required.** Path to the directory containing Axum controllers. | `None` |
| `output_file` | `Option<String>` | Output path for the generated Rust links enum. | `"links.rs"` |
| `generate_typescript_client` | `Option<bool>` | Enable TypeScript client generation. | `false` |
| `typescript_client_output` | `Option<PathBuf>` | Output path for the generated TypeScript client. | `None` |
| `include_method_in_names` | `Option<bool>` | Include the HTTP method in the generated variant names. | `false` |
| `path_prefix_to_remove` | `Option<String>` | Strip a specific prefix (e.g., `/api`) from generated paths. | `None` |
| `variant_case` | `Option<String>` | Casing for enum variants (e.g., `"PascalCase"`). | `"PascalCase"` |
| `field_case` | `Option<String>` | Casing for route parameter fields (e.g., `"snake_case"`). | `"snake_case"` |

> [!TIP]
> **Recommended Project Structure**
> ```text
> my-project/
> ├── Cargo.toml
> ├── build.rs
> ├── src/
> │   ├── controllers/      # Your Axum route definitions
> │   └── generated/        # Ignored in git, contains links.rs
> └── frontend/
>     └── src/api/          # Ignored in git, contains client.ts
> ```

## Generated Output Examples

### Rust Enum
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Link {
    GetUsers,
    GetUser { user_id: String },
    CreateUser,
    DeleteUser { user_id: String },
}

impl Link {
    pub fn to_path(&self) -> String {
        match self {
            Link::GetUsers => "/api/users".to_string(),
            Link::GetUser { user_id } => format!("/api/users/{}", user_id),
            // ...
        }
    }

    pub fn method(&self) -> &'static str {
        match self {
            Link::GetUsers => "GET",
            Link::GetUser { .. } => "GET",
            // ...
        }
    }
}
```

### TypeScript Client
```typescript
import { useQuery, useMutation } from "@tanstack/react-query";

export const client = {
  getUser: (params: { userId: string }) => ({
    url: `/api/users/${params.userId}`,
    method: 'GET',
  }),
};

export function useGetUser(params: { userId: string }, options?: any) {
  return useQuery({
    queryKey: ["getUser", params],
    queryFn: () => {
      const { url, method } = client.getUser(params);
      return fetch(url, { method }).then(res => res.json());
    },
    ...options,
  });
}
```

## Naming Conventions

The library uses the `convert_case` crate to transform route paths into idiomatic names. 

| Route | Method | Generated Name (PascalCase) |
| :--- | :--- | :--- |
| `/users` | GET | `GetUsers` |
| `/users/{id}` | GET | `GetUserById` |
| `/api/posts/{post_id}/comments` | POST | `CreatePostComment` |

> [!NOTE]
> Numbers are treated as separate words by default (e.g., `v1` becomes `V1`). This behavior can be toggled via the `preserve_numbers` configuration flag.

## Troubleshooting

> [!WARNING]
> **Common Issues**
> - **Routes not found**: Ensure your controller files expose a `routes()` function or follow the expected Axum `Router` construction pattern.
> - **Duplicate variants**: If multiple routes resolve to the same name, use `path_prefix_to_remove` or enable `include_method_in_names` to differentiate them. The build script will emit `cargo:warning` messages for skipped duplicates.
> - **Build errors**: Verify that all files in `controllers_path` contain valid Rust syntax, as the scanner parses them as ASTs.

## API Reference

- **`generate_links(config: &Config)`**: Main entry point. Scans controllers and generates both Rust and TypeScript outputs.
- **`generate_ts_client(config: &Config)`**: Generates only the TypeScript client.
- **`Config`**: Central configuration struct for path resolution, naming rules, and output targets.
- **`RouteInfo`**: Internal representation of a parsed route (name, path, method, parameters).
