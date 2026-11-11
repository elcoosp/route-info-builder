//! Build-time route scanner for Axum-like frameworks.
//!
//! `route-info-builder` scans controller files for `routes()`-style functions
//! and generates:
//!
//! * a type-safe Rust link enum (`to_path`, `method`, `name`),
//! * a typed TypeScript HTTP client with `TanStack` Query hooks,
//! * an `OpenAPI` 3.1 skeleton,
//! * a JSON route map.
//!
//! Both the Loco registration style
//! (`Routes::new().prefix("/api").add("/users", get(list_users))`) and the
//! plain Axum style (`Router::new().route(...)`, `.nest(...)`, `.merge(...)`)
//! are understood, with `{param}` (Axum 0.8) and legacy `:param` path syntax.
//!
//! # Quick start inside a `build.rs`
//!
//! ```no_run
//! use route_info_builder::{Config, RustConfig};
//!
//! let config = Config {
//!     controllers_path: "src/controllers".into(),
//!     rust: RustConfig {
//!         output_file: Some("src/generated/links.rs".into()),
//!         ..RustConfig::default()
//!     },
//!     ..Config::default()
//! };
//!
//! // Writes `src/generated/links.rs` and returns the generated code.
//! let code = route_info_builder::generate_links(&config)?;
//! println!("generated {} bytes of Rust", code.len());
//! # Ok::<(), route_info_builder::Error>(())
//! ```
//!
//! # End-to-end example
//!
//! ```no_run
//! use route_info_builder::{Config, RustConfig, TypeScriptConfig, scan_routes};
//! use std::path::PathBuf;
//!
//! let dir = tempfile::tempdir()?;
//! let controllers = dir.path().join("controllers");
//! std::fs::create_dir_all(&controllers)?;
//! std::fs::write(
//!     controllers.join("users.rs"),
//!     "pub fn routes() -> Router {
//!         Router::new()
//!             .route(\"/api/users\", get(list_users))
//!             .route(\"/api/users/{user_id}\", get(get_user))
//!     }",
//! )?;
//!
//! let config = Config {
//!     controllers_path: controllers.clone(),
//!     rust: RustConfig {
//!         output_file: Some(dir.path().join("out/links.rs")),
//!         ..RustConfig::default()
//!     },
//!     typescript: TypeScriptConfig {
//!         generate_client: Some(true),
//!         output_path: Some(dir.path().join("web/api")),
//!         ..TypeScriptConfig::default()
//!     },
//!     ..Config::default()
//! };
//!
//! let links = route_info_builder::generate_links(&config)?;
//! assert!(links.contains("pub enum Link"));
//! assert_eq!(scan_routes(&config)?.len(), 2);
//! // `out/links.rs` and `web/api/client.ts` + `web/api/api.ts` now exist.
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(test)]
mod test_support;

mod configuration;
mod error;
mod generators;
mod parser;
mod resolved;
mod ts;
mod utils;

use std::path::Path;

pub use configuration::{
    AuthConfig, CollisionStrategy, Config, NamingConfig, OpenApiConfig, RouteMapConfig, RustConfig,
    TypeScriptConfig,
};
pub use error::{Error, Result};
pub use generators::CodeGenerator;
pub use generators::openapi::OpenApiGenerator;
pub use generators::rust::LinksGenerator as RustLinksGenerator;
pub use generators::typescript::{
    ClientGenerator as TypeScriptClientGenerator, HooksGenerator as TypeScriptHooksGenerator,
};
pub use parser::{HandlerInfo, ReturnType, RouteInfo, scan_controllers_folder, scan_routes};
pub use resolved::{NamedRoute, resolve_route_names};

/// Scans the controllers and generates the Rust link enum.
///
/// When [`RustConfig::output_file`] is set the code is also written to disk
/// (parent directories are created). When
/// [`TypeScriptConfig::generate_client`] is enabled together with
/// [`TypeScriptConfig::output_path`], the TypeScript client and hooks files
/// are written next to it; `OpenAPI` and route-map outputs are written when
/// their output paths are configured.
///
/// # Errors
///
/// Returns an [`Error`] when controllers cannot be scanned or any output
/// file cannot be written.
pub fn generate_links(config: &Config) -> Result<String> {
    let routes = scan_routes(config)?;
    // Every artifact consumes the same resolved names, which keeps the Rust
    // `name()` values, the TypeScript client keys and the OpenAPI
    // `operationId`s in parity even when the naming configuration collapses
    // distinct routes onto one base name.
    let named = resolve_route_names(&routes, &config.naming);
    let rust_code = RustLinksGenerator::generate_named(&named, config)?;

    if let Some(output_file) = &config.rust.output_file {
        write_output(output_file, &rust_code)?;
    }

    if config.typescript.generate_client()
        && let Some(output_dir) = &config.typescript.output_path
    {
        let client = TypeScriptClientGenerator::generate_named(&named, &config.typescript)?;
        let hooks = TypeScriptHooksGenerator::generate_named(&named, &config.typescript)?;
        let client_path = output_dir.join(config.typescript_client_file());
        let hooks_path = output_dir.join(config.typescript_hooks_file());
        write_output(&client_path, &client)?;
        write_output(&hooks_path, &hooks)?;
        utils::build_warning(&format!(
            "route-info-builder: generated TypeScript client at {}",
            output_dir.display()
        ));
    }

    if let Some(openapi_path) = &config.openapi.output_path {
        let document = OpenApiGenerator::generate_named(&named, &config.openapi)?;
        write_output(openapi_path, &document)?;
    }

    if let Some(route_map_path) = &config.route_map.output_path {
        let route_map = route_map_json(&routes)?;
        write_output(route_map_path, &route_map)?;
    }

    Ok(rust_code)
}

/// Scans the controllers and returns only the TypeScript client code.
///
/// # Errors
///
/// Returns an [`Error`] when controllers cannot be scanned.
pub fn generate_ts_client(config: &Config) -> Result<String> {
    let routes = scan_routes(config)?;
    let named = resolve_route_names(&routes, &config.naming);
    TypeScriptClientGenerator::generate_named(&named, &config.typescript)
}

/// Scans the controllers and returns only the `TanStack` Query hooks code.
///
/// # Errors
///
/// Returns an [`Error`] when controllers cannot be scanned.
pub fn generate_ts_hooks(config: &Config) -> Result<String> {
    let routes = scan_routes(config)?;
    let named = resolve_route_names(&routes, &config.naming);
    TypeScriptHooksGenerator::generate_named(&named, &config.typescript)
}

/// Scans the controllers and returns an `OpenAPI` 3.1 skeleton as JSON.
///
/// # Errors
///
/// Returns an [`Error`] when controllers cannot be scanned or the document
/// cannot be serialized.
pub fn generate_openapi_json(config: &Config) -> Result<String> {
    let routes = scan_routes(config)?;
    let named = resolve_route_names(&routes, &config.naming);
    OpenApiGenerator::generate_named(&named, &config.openapi)
}

/// Scans the controllers and returns the JSON route map.
///
/// # Errors
///
/// Returns an [`Error`] when controllers cannot be scanned or the map
/// cannot be serialized.
pub fn generate_route_map_json(config: &Config) -> Result<String> {
    let routes = scan_routes(config)?;
    route_map_json(&routes)
}

/// Serializes scanned routes into a stable JSON route map.
fn route_map_json(routes: &[RouteInfo]) -> Result<String> {
    Ok(serde_json::to_string_pretty(routes)?)
}

/// Writes `content` to `path`, creating parent directories when needed.
fn write_output(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|source| Error::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, content).map_err(|source| Error::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use std::path::Path;

    use crate::configuration::{Config, RustConfig};

    /// Tests touch the filesystem, so they use a boxed error alias.
    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn fixture_config(dir: &std::path::Path) -> Config {
        let controllers = dir.join("controllers");
        std::fs::create_dir_all(&controllers).expect("create controllers dir");
        std::fs::write(
            controllers.join("users.rs"),
            "pub fn routes() -> Router {\n  Router::new()\n    .route(\"/api/users\", get(list_users))\n    .route(\"/api/users/{user_id}\", get(get_user))\n}\n",
        )
        .expect("write fixture");
        Config {
            controllers_path: controllers,
            ..Config::default()
        }
    }

    #[test]
    fn generate_links_writes_all_outputs() -> TestResult {
        let dir = tempfile::tempdir()?;
        let mut config = fixture_config(dir.path());
        config.rust = RustConfig {
            output_file: Some(dir.path().join("gen/links.rs")),
            ..RustConfig::default()
        };
        config.typescript.generate_client = Some(true);
        config.typescript.output_path = Some(dir.path().join("web/api"));
        config.openapi.output_path = Some(dir.path().join("openapi.json"));
        config.route_map.output_path = Some(dir.path().join("routes.json"));

        let code = crate::generate_links(&config)?;
        assert!(code.contains("pub enum Link"));
        assert!(dir.path().join("gen/links.rs").exists());
        assert!(dir.path().join("web/api/client.ts").exists());
        assert!(dir.path().join("web/api/api.ts").exists());
        assert!(dir.path().join("openapi.json").exists());
        assert!(dir.path().join("routes.json").exists());

        let route_map = std::fs::read_to_string(dir.path().join("routes.json"))?;
        assert!(route_map.contains("\"method\": \"GET\""));
        assert!(route_map.contains("\"handlerInfo\""));
        Ok(())
    }

    #[test]
    fn individual_generators_return_expected_content() -> TestResult {
        let dir = tempfile::tempdir()?;
        let config = fixture_config(dir.path());
        assert!(crate::generate_ts_client(&config)?.contains("export const client"));
        assert!(crate::generate_ts_hooks(&config)?.contains("export function use"));
        assert!(crate::generate_openapi_json(&config)?.contains("\"openapi\": \"3.1.0\""));
        assert!(crate::generate_route_map_json(&config)?.contains("\"path\": \"/api/users\""));
        assert_eq!(crate::scan_controllers_folder(&config)?.len(), 2);
        Ok(())
    }

    #[test]
    fn scan_missing_directory_propagates_error() {
        let config = Config {
            controllers_path: "/definitely/missing/controllers".into(),
            ..Config::default()
        };
        let error = crate::scan_routes(&config).expect_err("must fail");
        assert!(matches!(error, Error::ReadDir { .. }));
    }

    #[test]
    fn scan_invalid_rust_propagates_parse_error() -> TestResult {
        let dir = tempfile::tempdir()?;
        let config = fixture_config(dir.path());
        let bad_file = config.controllers_path.join("bad.rs");
        std::fs::write(&bad_file, "fn broken( {")?;
        let error = crate::scan_routes(&config).expect_err("must fail");
        assert!(
            matches!(&error, Error::Parse { path, .. } if *path == bad_file),
            "unexpected error: {error}"
        );
        Ok(())
    }

    #[test]
    fn write_output_handles_missing_parent_and_directories() -> TestResult {
        let dir = tempfile::tempdir()?;
        // Parent directories are created on demand.
        crate::write_output(&dir.path().join("a/b/c.txt"), "x")?;
        assert_eq!(std::fs::read_to_string(dir.path().join("a/b/c.txt"))?, "x");
        // Relative file names without a parent component work too.
        let previous = std::env::current_dir()?;
        let _guard = crate::test_support::CurrentDirGuard::lock()?;
        std::env::set_current_dir(dir.path())?;
        crate::write_output(Path::new("flat.txt"), "y")?;
        std::env::set_current_dir(previous)?;
        assert_eq!(std::fs::read_to_string(dir.path().join("flat.txt"))?, "y");
        // Writing onto a directory surfaces a WriteFile error.
        let error = crate::write_output(&dir.path().join("a"), "z").expect_err("must fail");
        assert!(matches!(error, Error::WriteFile { .. }));
        Ok(())
    }

    #[test]
    fn create_dir_error_is_surfaced() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("blocker"), "")?;
        let error = crate::write_output(&dir.path().join("blocker/inner/file.txt"), "x")
            .expect_err("must fail");
        assert!(matches!(error, Error::CreateDir { .. }));
        Ok(())
    }
}
