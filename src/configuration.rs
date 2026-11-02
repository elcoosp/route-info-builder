//! Configuration model for the route scanner and the generators.
//!
//! A [`Config`] can be built in code (every sub-struct implements
//! [`Default`]) or deserialized from a TOML file with
//! [`Config::from_toml_file`] / [`Config::from_toml_str`].

pub mod naming;

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

/// Top-level configuration.
///
/// Missing TOML fields fall back to their defaults; unknown keys are
/// rejected so that typos surface immediately.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Directory (recursively) scanned for controller files.
    pub controllers_path: PathBuf,
    /// Name of the function inside a controller file that builds the routes.
    /// Defaults to `routes`.
    pub controllers_fn: Option<String>,
    /// Naming rules for generated variant and field names.
    #[serde(default)]
    pub naming: NamingConfig,
    /// Rust `Link` enum generation options.
    #[serde(default)]
    pub rust: RustConfig,
    /// TypeScript client and hooks generation options.
    #[serde(default)]
    pub typescript: TypeScriptConfig,
    /// `OpenAPI` 3.1 skeleton generation options.
    #[serde(default)]
    pub openapi: OpenApiConfig,
    /// JSON route map generation options.
    #[serde(default)]
    pub route_map: RouteMapConfig,
}

/// Strategy applied when two routes would map to the same enum variant name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CollisionStrategy {
    /// Keep the first route, warn about and skip the duplicates.
    #[default]
    Skip,
    /// Prefix the HTTP method to the duplicate (`GetUsers` versus `PostUsers`).
    AppendMethod,
    /// Append a counter to the duplicate (`Users`, `Users2`, ...).
    Number,
}

/// Naming rules applied while generating variant and field names.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamingConfig {
    /// Include the HTTP method in the generated base names (default `true`).
    pub include_method_in_names: Option<bool>,
    /// Path prefix removed from generated *names* (kept in the URL paths).
    pub path_prefix_to_remove: Option<String>,
    /// Case for variant names (default `pascal`).
    pub variant_case: Option<String>,
    /// Case for route parameter fields (default `snake`).
    pub field_case: Option<String>,
    /// Extra characters treated as word separators.
    pub word_separators: Option<String>,
    /// When `false`, digits stay glued to the preceding word (`v1` instead of
    /// `v_1` in snake case).
    pub preserve_numbers: Option<bool>,
    /// Static prefix prepended to every variant name.
    pub variant_prefix: Option<String>,
    /// Static suffix appended to every variant name.
    pub variant_suffix: Option<String>,
    /// Strategy used when two routes produce the same variant name.
    pub collision_strategy: Option<CollisionStrategy>,
}

/// Rust generation options.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustConfig {
    /// When set, the generated enum is also written to this file
    /// (parent directories are created).
    pub output_file: Option<PathBuf>,
    /// Name of the generated enum (default `Link`).
    pub enum_name: Option<String>,
    /// Add `serde::Serialize`/`serde::Deserialize` derives to the enum.
    pub derive_serde: Option<bool>,
}

/// Authentication plumbing for the generated TypeScript client.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    /// Name of the exported token key constant (default `TOKEN_KEY`).
    pub token_key_import: Option<String>,
    /// Module the token key constant is imported from, e.g. `@/hooks/use-auth`.
    pub import_from: Option<String>,
}

/// TypeScript generation options.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeScriptConfig {
    /// Generate the client and hooks files (default `false`).
    pub generate_client: Option<bool>,
    /// Directory receiving `client.ts` and the hooks file.
    pub output_path: Option<PathBuf>,
    /// File name of the generated client (default `client.ts`).
    pub client_file: Option<String>,
    /// File name of the generated hooks (default `api.ts`).
    pub hooks_file: Option<String>,
    /// Import base path for generated binding types
    /// (default `../../../bindings`).
    pub bindings_path: Option<String>,
    /// Authentication plumbing; omit to generate an anonymous client.
    pub auth: Option<AuthConfig>,
}

/// `OpenAPI` generation options.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenApiConfig {
    /// When set, an `OpenAPI` 3.1 skeleton is written to this file.
    pub output_path: Option<PathBuf>,
    /// API title used in the document (default `API`).
    pub title: Option<String>,
    /// API version used in the document (default `0.1.0`).
    pub version: Option<String>,
}

/// Route map generation options.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteMapConfig {
    /// When set, the JSON route map is written to this file.
    pub output_path: Option<PathBuf>,
}

impl Config {
    /// Parses a configuration from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] when the TOML is invalid or contains
    /// unknown keys.
    pub fn from_toml_str(content: &str) -> Result<Self> {
        toml::from_str(content).map_err(Error::Config)
    }

    /// Parses a configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ReadFile`] when the file cannot be read and
    /// [`Error::Config`] when its content is invalid.
    pub fn from_toml_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|source| Error::ReadFile {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml_str(&content)
    }

    /// Loads the configuration from `path`, or from `route-info.toml` in the
    /// working directory when `path` is [`None`] and that file exists.
    ///
    /// # Errors
    ///
    /// Same as [`Config::from_toml_file`].
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let Some(explicit) = path else {
            let default_path = Path::new("route-info.toml");
            return if default_path.exists() {
                Self::from_toml_file(default_path)
            } else {
                Ok(Self::default())
            };
        };
        Self::from_toml_file(explicit)
    }

    /// Function name looked up inside controller files.
    #[must_use]
    pub fn controllers_function(&self) -> &str {
        self.controllers_fn.as_deref().unwrap_or("routes")
    }

    /// File name of the generated TypeScript client.
    #[must_use]
    pub fn typescript_client_file(&self) -> &str {
        self.typescript
            .client_file
            .as_deref()
            .unwrap_or("client.ts")
    }

    /// File name of the generated TypeScript hooks.
    #[must_use]
    pub fn typescript_hooks_file(&self) -> &str {
        self.typescript.hooks_file.as_deref().unwrap_or("api.ts")
    }
}

impl NamingConfig {
    /// Whether generated base names include the HTTP method.
    #[must_use]
    pub fn include_method(&self) -> bool {
        self.include_method_in_names.unwrap_or(true)
    }

    /// Case used for variant names.
    #[must_use]
    pub fn variant_case(&self) -> &str {
        self.variant_case.as_deref().unwrap_or("pascal")
    }

    /// Case used for parameter field names.
    #[must_use]
    pub fn field_case(&self) -> &str {
        self.field_case.as_deref().unwrap_or("snake")
    }

    /// Strategy applied to variant name collisions.
    #[must_use]
    pub fn collision_strategy(&self) -> CollisionStrategy {
        self.collision_strategy.unwrap_or_default()
    }
}

impl RustConfig {
    /// Name of the generated enum.
    #[must_use]
    pub fn enum_name(&self) -> &str {
        self.enum_name.as_deref().unwrap_or("Link")
    }

    /// Whether the generated enum receives serde derives.
    #[must_use]
    pub fn derive_serde(&self) -> bool {
        self.derive_serde.unwrap_or(false)
    }
}

impl TypeScriptConfig {
    /// Whether TypeScript output is requested.
    #[must_use]
    pub fn generate_client(&self) -> bool {
        self.generate_client.unwrap_or(false)
    }

    /// Base path used when importing binding types.
    #[must_use]
    pub fn bindings_path(&self) -> &str {
        self.bindings_path.as_deref().unwrap_or("../../../bindings")
    }

    /// Imported name of the token key constant, if authentication is wired.
    #[must_use]
    pub fn auth_token_key(&self) -> Option<&str> {
        self.auth
            .as_ref()
            .map(|auth| auth.token_key_import.as_deref().unwrap_or("TOKEN_KEY"))
    }

    /// Module the token key constant is imported from, if configured.
    #[must_use]
    pub fn auth_import_from(&self) -> Option<&str> {
        self.auth
            .as_ref()
            .and_then(|auth| auth.import_from.as_deref())
    }
}

impl OpenApiConfig {
    /// Title placed in the `OpenAPI` document.
    #[must_use]
    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or("API")
    }

    /// Version placed in the `OpenAPI` document.
    #[must_use]
    pub fn version(&self) -> &str {
        self.version.as_deref().unwrap_or("0.1.0")
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::{
        CollisionStrategy, Config, NamingConfig, OpenApiConfig, RouteMapConfig, RustConfig,
        TypeScriptConfig,
    };

    /// Tests touch the filesystem, so they use a boxed error alias.
    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn default_config_has_expected_fallbacks() {
        let config = Config::default();
        assert_eq!(config.controllers_function(), "routes");
        assert_eq!(config.typescript_client_file(), "client.ts");
        assert_eq!(config.typescript_hooks_file(), "api.ts");
        assert!(config.naming.include_method());
        assert_eq!(config.naming.variant_case(), "pascal");
        assert_eq!(config.naming.field_case(), "snake");
        assert_eq!(config.naming.collision_strategy(), CollisionStrategy::Skip);
        assert_eq!(config.rust.enum_name(), "Link");
        assert!(!config.rust.derive_serde());
        assert!(!config.typescript.generate_client());
        assert_eq!(config.typescript.bindings_path(), "../../../bindings");
        assert!(config.typescript.auth_token_key().is_none());
        assert!(config.typescript.auth_import_from().is_none());
        assert_eq!(config.openapi.title(), "API");
        assert_eq!(config.openapi.version(), "0.1.0");
    }

    #[test]
    fn from_toml_str_parses_full_config() -> TestResult {
        let toml = r#"
            controllers_path = "src/controllers"
            controllers_fn = "api_routes"
            [naming]
            include_method_in_names = false
            path_prefix_to_remove = "/api"
            variant_case = "pascal"
            field_case = "camel"
            word_separators = "-/"
            preserve_numbers = false
            variant_prefix = "Api"
            variant_suffix = "Route"
            collision_strategy = "append-method"
            [rust]
            output_file = "src/links.rs"
            enum_name = "ApiLink"
            derive_serde = true
            [typescript]
            generate_client = true
            output_path = "web/src/api"
            client_file = "api-client.ts"
            hooks_file = "api-hooks.ts"
            bindings_path = "../bindings"
            [typescript.auth]
            token_key_import = "AUTH_TOKEN"
            import_from = "@/lib/auth"
            [openapi]
            output_path = "openapi.json"
            title = "My API"
            version = "1.2.3"
            [route_map]
            output_path = "routes.json"
        "#;
        let config = Config::from_toml_str(toml)?;
        assert_eq!(config.controllers_path, PathBuf::from("src/controllers"));
        assert_eq!(config.controllers_function(), "api_routes");
        assert!(!config.naming.include_method());
        assert_eq!(config.naming.path_prefix_to_remove.as_deref(), Some("/api"));
        assert_eq!(config.naming.field_case(), "camel");
        assert_eq!(config.naming.word_separators.as_deref(), Some("-/"));
        assert_eq!(config.naming.preserve_numbers, Some(false));
        assert_eq!(config.naming.variant_prefix.as_deref(), Some("Api"));
        assert_eq!(config.naming.variant_suffix.as_deref(), Some("Route"));
        assert_eq!(
            config.naming.collision_strategy(),
            CollisionStrategy::AppendMethod
        );
        assert_eq!(config.rust.output_file, Some(PathBuf::from("src/links.rs")));
        assert_eq!(config.rust.enum_name(), "ApiLink");
        assert!(config.rust.derive_serde());
        assert!(config.typescript.generate_client());
        assert_eq!(
            config.typescript.output_path,
            Some(PathBuf::from("web/src/api"))
        );
        assert_eq!(config.typescript_client_file(), "api-client.ts");
        assert_eq!(config.typescript_hooks_file(), "api-hooks.ts");
        assert_eq!(config.typescript.bindings_path(), "../bindings");
        assert_eq!(config.typescript.auth_token_key(), Some("AUTH_TOKEN"));
        assert_eq!(config.typescript.auth_import_from(), Some("@/lib/auth"));
        assert_eq!(config.openapi.title(), "My API");
        assert_eq!(config.openapi.version(), "1.2.3");
        assert_eq!(
            config.route_map.output_path,
            Some(PathBuf::from("routes.json"))
        );
        Ok(())
    }

    #[test]
    fn from_toml_str_accepts_minimal_config() -> TestResult {
        let config = Config::from_toml_str("controllers_path = \"src/controllers\"")?;
        assert_eq!(config.controllers_path, PathBuf::from("src/controllers"));
        assert!(config.rust.output_file.is_none());
        Ok(())
    }

    #[test]
    fn from_toml_str_rejects_unknown_keys() {
        let error = Config::from_toml_str("controllers_path = \"x\"\nnaming_ohno = true")
            .expect_err("unknown key must be rejected");
        assert!(error.to_string().starts_with("invalid configuration:"));
    }

    #[test]
    fn from_toml_str_rejects_unknown_naming_keys() {
        let error =
            Config::from_toml_str("[naming]\nnope = 1").expect_err("unknown key must be rejected");
        assert!(error.to_string().starts_with("invalid configuration:"));
    }

    #[test]
    fn from_toml_str_rejects_invalid_strategy() {
        let error = Config::from_toml_str(
            "controllers_path = \"x\"\n[naming]\ncollision_strategy = \"explode\"",
        )
        .expect_err("invalid strategy must be rejected");
        assert!(error.to_string().starts_with("invalid configuration:"));
    }

    #[test]
    fn from_toml_file_reads_and_parses() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("route-info.toml");
        std::fs::write(&path, "controllers_path = \"src/controllers\"")?;
        let config = Config::from_toml_file(&path)?;
        assert_eq!(config.controllers_path, PathBuf::from("src/controllers"));
        Ok(())
    }

    #[test]
    fn load_reads_default_file_when_present() -> TestResult {
        let cwd_guard = crate::test_support::CurrentDirGuard::lock()?;
        let dir = tempfile::tempdir()?;
        std::env::set_current_dir(dir.path())?;
        std::fs::write("route-info.toml", "controllers_path = \"in/cwd\"")?;
        let loaded = Config::load(None)?;
        drop(cwd_guard);
        assert_eq!(loaded.controllers_path, PathBuf::from("in/cwd"));
        Ok(())
    }

    #[test]
    fn load_falls_back_to_default_without_file() -> TestResult {
        let cwd_guard = crate::test_support::CurrentDirGuard::lock()?;
        let dir = tempfile::tempdir()?;
        std::env::set_current_dir(dir.path())?;
        let loaded = Config::load(None)?;
        drop(cwd_guard);
        assert_eq!(loaded.controllers_path, Config::default().controllers_path);
        Ok(())
    }

    #[test]
    fn load_prefers_explicit_path() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("custom.toml");
        std::fs::write(&path, "controllers_path = \"custom\"")?;
        let loaded = Config::load(Some(&path))?;
        assert_eq!(loaded.controllers_path, PathBuf::from("custom"));
        Ok(())
    }

    #[test]
    fn load_missing_explicit_path_errors() {
        let error = Config::load(Some(Path::new("/definitely/missing/route-info.toml")))
            .expect_err("missing file must fail");
        assert!(error.to_string().contains("failed to read controller file"));
    }

    #[test]
    fn struct_field_configs_are_debuggable() {
        // Coverage: exercise the Debug impls of the plain configuration structs.
        let debug = format!(
            "{:?}{:?}{:?}{:?}{:?}",
            NamingConfig::default(),
            RustConfig::default(),
            TypeScriptConfig::default(),
            OpenApiConfig::default(),
            RouteMapConfig::default(),
        );
        assert!(debug.contains("NamingConfig"));
        assert!(debug.contains("RouteMapConfig"));
    }

    #[test]
    fn collision_strategy_is_copyable_and_comparable() {
        let strategy = CollisionStrategy::Number;
        let copied = strategy;
        assert_eq!(strategy, copied);
        assert_ne!(strategy, CollisionStrategy::Skip);
    }
}
