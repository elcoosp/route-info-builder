pub mod naming;
use serde::Deserialize;
use std::path::PathBuf;
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub controllers_path: PathBuf,
    pub naming: NamingConfig,
    pub typescript: TypeScriptConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct NamingConfig {
    /// Whether to include HTTP methods in variant names
    pub include_method_in_names: Option<bool>,
    /// Custom prefix to remove from paths when generating names
    pub path_prefix_to_remove: Option<String>,
    /// Case for variant names (default: "PascalCase")
    pub variant_case: Option<String>,
    /// Case for field names (default: "snake_case")
    pub field_case: Option<String>,
    /// Characters to treat as word separators in route names
    pub word_separators: Option<String>,
    /// Whether to preserve numbers as separate words
    pub preserve_numbers: Option<bool>,
    /// Custom prefix for variant names
    pub variant_prefix: Option<String>,
    /// Custom suffix for variant names
    pub variant_suffix: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TypeScriptConfig {
    /// Optional output path for TypeScript client
    pub output_path: Option<PathBuf>,
    /// Whether to generate TypeScript client
    pub generate_client: Option<bool>,
}
