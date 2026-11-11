//! Scans controller files and extracts [`RouteInfo`] entries.

pub(crate) mod handlers;
pub(crate) mod routes;

pub use handlers::ReturnType;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::configuration::Config;
use crate::error::{Error, Result};
use crate::utils::{build_warning, path};

/// A single scanned route with its naming and handler metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteInfo {
    /// Base name generated from the path and the HTTP method.
    pub name: String,
    /// Normalized URL path with `{param}` placeholders.
    pub path: String,
    /// Upper-case HTTP method, e.g. `GET`.
    pub method: String,
    /// Name of the handler function servicing the route.
    pub handler: String,
    /// Extra information extracted from the handler function.
    pub handler_info: HandlerInfo,
}

/// Information extracted from the handler function behind a route.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandlerInfo {
    /// Inner type of a `Json<T>`-like body extractor.
    pub body_param: Option<String>,
    /// Inner type of a `Query<T>` extractor.
    pub query_params: Option<String>,
    /// Whether the handler takes an `auth: JWT` extractor.
    pub requires_auth: bool,
    /// Return type information recovered from the handler body.
    pub return_type: handlers::ReturnType,
}

/// Scans the controllers folder recursively (alias of [`scan_routes`],
/// kept for backward compatibility).
///
/// # Errors
///
/// Returns an [`Error`] when the directory cannot be read, a file cannot be
/// read or a controller file contains invalid Rust.
pub fn scan_controllers_folder(config: &Config) -> Result<Vec<RouteInfo>> {
    scan_routes(config)
}

/// Scans the controllers folder recursively and returns all routes.
///
/// * only `*.rs` files are considered; `mod.rs`, dot-files and dot-directories
///   are skipped;
/// * files are visited in sorted order so the output is deterministic;
/// * duplicate `(method, path)` pairs are dropped with a `cargo:warning`;
/// * routes with the same path shape but different parameter names also
///   produce a warning.
///
/// # Errors
///
/// Returns an [`Error`] when the directory cannot be read, a file cannot be
/// read or a controller file contains invalid Rust.
pub fn scan_routes(config: &Config) -> Result<Vec<RouteInfo>> {
    let files = collect_controller_files(&config.controllers_path)?;
    let mut routes = Vec::new();
    for file in files {
        routes.extend(parse_routes_from_file(&file, config)?);
    }
    deduplicate_routes(&mut routes);
    warn_on_inconsistent_parameters(&routes);
    if routes.is_empty() {
        build_warning(&format!(
            "route-info-builder: no routes found under {}",
            config.controllers_path.display()
        ));
    }
    Ok(routes)
}

/// Collects controller files recursively, in sorted order.
fn collect_controller_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    walk_directory(dir, &mut files)?;
    files.sort();
    Ok(files)
}

/// Recursively appends controller files from `dir` into `files`.
fn walk_directory(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|source| Error::ReadDir {
        path: dir.to_path_buf(),
        source,
    })?;
    let mut paths = Vec::new();
    for entry in entries.flatten() {
        paths.push(entry.path());
    }
    for entry_path in paths {
        let Some(name) = entry_path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if entry_path.is_dir() {
            walk_directory(&entry_path, files)?;
        } else if name.to_lowercase().ends_with(".rs") && name != "mod.rs" {
            files.push(entry_path);
        }
    }
    Ok(())
}

/// Parses one controller file into its routes (empty when it has none).
fn parse_routes_from_file(path: &Path, config: &Config) -> Result<Vec<RouteInfo>> {
    let content = fs::read_to_string(path).map_err(|source| Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    let syntax = syn::parse_file(&content).map_err(|source| Error::Parse {
        path: path.to_path_buf(),
        source,
    })?;

    let routes_function = config.controllers_function();
    let routes_function = syntax.items.iter().find_map(|item| match item {
        syn::Item::Fn(func) if func.sig.ident == routes_function => Some(func),
        _ => None,
    });

    let Some(routes_function) = routes_function else {
        return Ok(Vec::new());
    };

    let mut routes = routes::extract(routes_function, config)?;
    let handler_info = handlers::extract_handler_info(&syntax);
    for route in &mut routes {
        if let Some(info) = handler_info.get(&route.handler) {
            route.handler_info = info.clone();
        }
    }
    Ok(routes)
}

/// Drops duplicate `(method, path)` routes, warning for each dropped entry.
fn deduplicate_routes(routes: &mut Vec<RouteInfo>) {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    routes.retain(|route| {
        if seen.insert((route.method.clone(), route.path.clone())) {
            true
        } else {
            build_warning(&format!(
                "route-info-builder: duplicate route skipped: {} {}",
                route.method, route.path
            ));
            false
        }
    });
}

/// Warns when routes sharing the same path shape use different parameter
/// names at the same position (e.g. `/users/{id}` and `/users/{user_id}`).
fn warn_on_inconsistent_parameters(routes: &[RouteInfo]) {
    let mut shapes: BTreeMap<String, Vec<BTreeSet<String>>> = BTreeMap::new();
    for route in routes {
        let (shape, names) = shape_and_names(&route.path);
        let positions = shapes.entry(shape).or_default();
        while positions.len() < names.len() {
            positions.push(BTreeSet::new());
        }
        for (position, name) in names.iter().enumerate() {
            positions[position].insert(name.clone());
        }
    }
    for (shape, positions) in shapes {
        for position in positions {
            if position.len() > 1 {
                let joined = position
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                build_warning(&format!(
                    "route-info-builder: path `{shape}` mixes parameter names: {joined}"
                ));
            }
        }
    }
}

/// Replaces parameter segments with indexed placeholders so paths can be
/// grouped by shape; returns the placeholder path and the parameter names in
/// order of appearance.
fn shape_and_names(path: &str) -> (String, Vec<String>) {
    let mut names: Vec<String> = Vec::new();
    let segments: Vec<String> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| match path::param_name(segment) {
            Some(name) => {
                let index = names.len();
                names.push(name.to_string());
                format!("{{{index}}}")
            }
            None => segment.to_string(),
        })
        .collect();
    (format!("/{}", segments.join("/")), names)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{collect_controller_files, shape_and_names};
    use crate::configuration::Config;
    use crate::error::Error;

    /// Tests touch the filesystem, so they use a boxed error alias.
    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn write(dir: &Path, relative: &str, content: &str) -> std::io::Result<()> {
        let path = dir.join(relative);
        let parent = path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    #[test]
    fn collect_files_is_recursive_sorted_and_skips_excluded() -> TestResult {
        let dir = tempfile::tempdir()?;
        write(dir.path(), "users.rs", "")?;
        write(dir.path(), "nested/posts.rs", "")?;
        write(dir.path(), "mod.rs", "")?;
        write(dir.path(), "notes.txt", "")?;
        write(dir.path(), ".hidden.rs", "")?;
        write(dir.path(), "nested/.secret.rs", "")?;
        let config = Config {
            controllers_path: dir.path().to_path_buf(),
            ..Config::default()
        };
        let files = collect_controller_files(&config.controllers_path)?;
        let names: Vec<String> = files
            .iter()
            .map(|f| f.strip_prefix(dir.path()).unwrap().display().to_string())
            .collect();
        assert_eq!(names, vec!["nested/posts.rs", "users.rs"]);
        Ok(())
    }

    #[test]
    fn unreadable_files_surface_read_errors() -> TestResult {
        #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir()?;
        let locked = dir.path().join("locked.rs");
        std::fs::write(&locked, "fn routes() -> Router { Router::new() }")?;
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))?;
        let config = Config {
            controllers_path: dir.path().to_path_buf(),
            ..Config::default()
        };
        let error = crate::scan_routes(&config).expect_err("unreadable file must fail");
        assert!(
            matches!(&error, Error::ReadFile { path, .. } if *path == locked),
            "unexpected error: {error}"
        );
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644))?;
        Ok(())
    }

    #[test]
    #[cfg(all(unix, not(target_os = "macos")))]
    fn non_utf8_file_names_are_skipped() -> TestResult {
        use std::os::unix::ffi::OsStrExt;

        let dir = tempfile::tempdir()?;
        let weird = dir
            .path()
            .join(std::ffi::OsStr::from_bytes(b"weird-\xff.rs"));
        std::fs::write(&weird, "fn routes() -> Router { Router::new() }")?;
        let normal = dir.path().join("normal.rs");
        std::fs::write(&normal, "fn routes() -> Router { Router::new() }")?;
        let config = Config {
            controllers_path: dir.path().to_path_buf(),
            ..Config::default()
        };
        let files = collect_controller_files(&config.controllers_path)?;
        assert_eq!(files, vec![normal]);
        Ok(())
    }

    #[test]
    fn collect_files_missing_dir_errors() {
        let error =
            collect_controller_files(Path::new("/definitely/not/here")).expect_err("must fail");
        assert!(
            error
                .to_string()
                .contains("failed to read controllers directory")
        );
    }

    #[test]
    fn shape_and_names_groups_by_position() {
        let (shape, names) = shape_and_names("/users/{id}/posts/{post_id}");
        assert_eq!(shape, "/users/{0}/posts/{1}");
        assert_eq!(names, vec!["id".to_string(), "post_id".to_string()]);
        let (legacy, legacy_names) = shape_and_names("/users/:id");
        assert_eq!(legacy, "/users/{0}");
        assert_eq!(legacy_names, vec!["id".to_string()]);
    }

    #[test]
    fn only_the_routes_function_is_scanned() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("mixed.rs");
        let fixture = "fn helper_routes() -> Router { Router::new().route(\"/nope\", get(nope)) }\n\
             fn routes() -> Router { Router::new().route(\"/yes\", get(yes)) }\n";
        std::fs::write(&path, fixture).expect("write fixture");
        let config = Config {
            controllers_path: dir.path().to_path_buf(),
            ..Config::default()
        };
        let scanned = super::parse_routes_from_file(&path, &config)?;
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].path, "/yes");

        let custom = Config {
            controllers_path: dir.path().to_path_buf(),
            controllers_fn: Some("helper_routes".to_string()),
            ..Config::default()
        };
        let scanned = super::parse_routes_from_file(&path, &custom)?;
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].path, "/nope");

        let empty_path = dir.path().join("empty.rs");
        std::fs::write(&empty_path, "fn unrelated() -> i32 { 42 }")?;
        assert_eq!(
            super::parse_routes_from_file(&empty_path, &config)?,
            Vec::new()
        );
        Ok(())
    }
}
