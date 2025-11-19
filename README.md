# Route Info Builder

A Rust procedural macro library for generating type-safe route information and TypeScript clients from Axum web framework routes.

## Features

- 🔍 **Automatic Route Scanning** - Scans your Axum controller files to extract route information
- 🦀 **Rust Enum Generation** - Generates type-safe Rust enums for all your application routes
- 📘 **TypeScript Client** - Creates a fully-typed TypeScript client with React Query hooks
- 🎯 **Type Safety** - Compile-time checked routes with parameters
- ⚡ **TanStack Query Integration** - Pre-built React hooks for data fetching and mutations
- 🔧 **Highly Configurable** - Customize naming conventions, cases, and output formats

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
route-info-builder = "0.1"
```

## Quick Start

### 1. Configure Your Build Script

Create a `build.rs` file in your project root:

```rust
use route_info_builder::Config;

fn main() {
    let config = Config {
        controllers_path: std::path::PathBuf::from("src/controllers"),
        output_file: Some("src/links.rs".to_string()),
        generate_typescript_client: Some(true),
        typescript_client_output: Some(std::path::PathBuf::from("frontend/src/api/client.ts")),
        ..Default::default()
    };

    route_info_builder::generate_links(&config).unwrap();
}
```

### 2. Use Generated Links in Rust

```rust
// Generated links enum
use crate::links::Link;

// Type-safe route usage
let user_link = Link::GetUsers { user_id: "123".to_string() };
println!("Path: {}", user_link.to_path()); // "/users/123"
println!("Method: {}", user_link.method()); // "GET"

// In Axum routes
app.route(&user_link.to_path(), get(handler))
    .route(&another_link.to_path(), post(handler));
```

### 3. Use TypeScript Client in Frontend

```typescript
import { useGetUsers, useCreateUser } from './api/client';

// React component
function UserList() {
  const { data: users, isLoading } = useGetUsers();
  const createUser = useCreateUser();

  if (isLoading) return <div>Loading...</div>;

  return (
    <div>
      {users.map(user => (
        <div key={user.id}>{user.name}</div>
      ))}
      <button onClick={() => createUser.mutate({ name: 'New User' })}>
        Create User
      </button>
    </div>
  );
}
```

## Configuration

### Basic Configuration

```rust
let config = Config {
    controllers_path: PathBuf::from("src/controllers"),
    output_file: Some("src/links.rs".to_string()),
    ..Default::default()
};
```

### Full Configuration Options

```rust
let config = Config {
    // Required: Path to controllers directory
    controllers_path: PathBuf::from("src/controllers"),
    
    // Optional: Output file for Rust links (default: "links.rs")
    output_file: Some("src/generated/links.rs".to_string()),
    
    // Naming options
    include_method_in_names: Some(true), // Include HTTP method in variant names
    path_prefix_to_remove: Some("/api".to_string()), // Remove prefix from paths
    variant_case: Some("PascalCase".to_string()), // Case for enum variants
    field_case: Some("snake_case".to_string()), // Case for field names
    word_separators: Some("-_".to_string()), // Characters treated as word separators
    
    // TypeScript generation
    generate_typescript_client: Some(true),
    typescript_client_output: Some(PathBuf::from("frontend/src/api.ts")),
    
    // Advanced naming
    variant_prefix: Some("".to_string()), // Prefix for variant names
    variant_suffix: Some("".to_string()), // Suffix for variant names
    preserve_numbers: Some(true), // Treat numbers as separate words
};
```

## Project Structure

### Recommended Layout

```
my-project/
├── Cargo.toml
├── build.rs
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── controllers/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   └── post.rs
│   └── generated/
│       └── links.rs  # Generated file
└── frontend/
    └── src/
        └── api/
            └── client.ts  # Generated TypeScript client
```

### Example Controller

```rust
// src/controllers/user.rs
use axum::{
    routing::{get, post, delete},
    Router,
};

pub fn routes() -> Router {
    Router::new()
        .prefix("/api")
        .add("/users", get(get_users))
        .add("/users/{user_id}", get(get_user))
        .add("/users", post(create_user))
        .add("/users/{user_id}", delete(delete_user))
}

async fn get_users() -> &'static str { "Get all users" }
async fn get_user() -> &'static str { "Get user" }
async fn create_user() -> &'static str { "Create user" }
async fn delete_user() -> &'static str { "Delete user" }
```

## Generated Output

### Rust Enum Example

```rust
/// Auto-generated link enum for all application routes
#[derive(Debug, Clone, PartialEq)]
pub enum Link {
    GetUsers,
    GetUser { user_id: String },
    CreateUser,
    DeleteUser { user_id: String },
}

impl Link {
    /// Convert the link to a URL path string
    pub fn to_path(&self) -> String {
        match self {
            Link::GetUsers => "/api/users".to_string(),
            Link::GetUser { user_id } => format!("/api/users/{}", user_id),
            // ... other variants
        }
    }

    /// Get the HTTP method for this route
    pub fn method(&self) -> &'static str {
        match self {
            Link::GetUsers => "GET",
            Link::GetUser { .. } => "GET",
            // ... other variants
        }
    }
}
```

### TypeScript Client Example

```typescript
// Generated TypeScript client with React Query hooks
import { useQuery, useMutation } from "@tanstack/react-query";

export const client = {
  getUsers: () => ({
    url: `/api/users`,
    method: 'GET',
  }),
  getUser: (params: GetUserParams) => ({
    url: `/api/users/${params.userId}`,
    method: 'GET',
  }),
  // ... other methods
};

// Hooks for React components
export function useGetUsers(options?: Omit<UseQueryOptions<any, Error>, "queryKey">) {
  return useQuery({
    queryKey: ["getUsers"],
    queryFn: () => {
      const { url, method } = client.getUsers();
      return fetch(url, { method }).then(res => res.json());
    },
    ...options,
  });
}
```

## Naming Conventions

The library supports multiple naming conventions through the `convert_case` crate:

### Supported Cases
- **PascalCase**: `GetUserById`
- **camelCase**: `getUserById` 
- **snake_case**: `get_user_by_id`
- **kebab-case**: `get-user-by-id`
- **Title Case**: `Get User By Id`

### Route to Name Conversion

| Route | Method | Generated Name (PascalCase) |
|-------|--------|----------------------------|
| `/users` | GET | `GetUsers` |
| `/users/{id}` | GET | `GetUserById` |
| `/api/posts/{post_id}/comments` | POST | `CreatePostComment` |

## Advanced Usage

### Custom Route Parameters

```rust
// Handle complex parameter types
let link = Link::SearchUsers { 
    query: "john".to_string(),
    page: "1".to_string(),
    limit: "10".to_string() 
};
```

### Integration with Frontend Frameworks

The generated TypeScript client works with:
- **React** (with TanStack Query)
- **Vue** (with Vue Query)
- **Svelte** (with Svelte Query)
- **Any framework** (raw client methods available)

### Error Handling

Duplicate routes are automatically detected and warnings are emitted during build:

```
cargo:warning=Duplicate route skipped: GET /api/users
```

## Troubleshooting

### Common Issues

1. **Routes not found**: Ensure your controller files have a `routes()` function
2. **Build errors**: Check that all controller files are valid Rust syntax
3. **Duplicate variants**: Use `path_prefix_to_remove` or adjust naming configuration

### Debugging

Enable verbose output by checking the cargo warnings:

```
cargo:warning=Generated Rust links enum at: src/links.rs
cargo:warning=Generated TypeScript client at: frontend/src/api.ts
```

## API Reference

### Main Functions

- `generate_links(config: &Config)` - Main function to generate both Rust and TypeScript outputs
- `generate_ts_client(config: &Config)` - Generate only TypeScript client

### Data Structures

- `Config` - Configuration for route scanning and code generation
- `RouteInfo` - Information about a single route (name, path, method)

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
