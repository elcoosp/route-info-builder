mod config;
mod generators;
mod parser;
mod utils;
use std::path::PathBuf;

use crate::generators::CodeGenerator;
pub use config::{Config, NamingConfig, TypeScriptConfig};
pub use generators::{
    rust::RustLinksGenerator,
    typescript::{TypeScriptClientGenerator, TypeScriptHooksGenerator},
};
pub use parser::{HandlerInfo, RouteInfo, scan_controllers_folder};
pub use utils::{case, path};

/// Main function to generate links enum from controller files
pub fn generate_links(config: &Config) -> Result<String, Box<dyn std::error::Error>> {
    let routes = scan_controllers_folder(config)?;

    // Generate Rust links enum
    let rust_code = RustLinksGenerator::generate(&routes, config)?;

    // Generate TypeScript client if requested
    if config.typescript.generate_client.unwrap_or(false)
        && let Some(ts_output) = &config.typescript.output_path
    {
        let ts_client_code = TypeScriptClientGenerator::generate(&routes, &config.typescript)?;
        let ts_hooks_code = TypeScriptHooksGenerator::generate(&routes, &config.typescript)?;

        // Combine client and hooks
        let ts_client_code_path = PathBuf::from(ts_output).join("client.ts");
        let ts_hooks_code_path = PathBuf::from(ts_output).join("api.ts");
        std::fs::write(ts_client_code_path, ts_client_code)?;
        std::fs::write(ts_hooks_code_path, ts_hooks_code)?;
        println!(
            "cargo:warning=Generated TypeScript client at: {}",
            ts_output.display()
        );
    }

    Ok(rust_code)
}

/// Generate TypeScript HTTP client compatible with tanstack-query
pub fn generate_ts_client(config: &Config) -> Result<String, Box<dyn std::error::Error>> {
    let routes = scan_controllers_folder(config)?;
    let ts_code = TypeScriptClientGenerator::generate(&routes, &config.typescript)?;
    Ok(ts_code)
}

/// Generate TypeScript hooks for tanstack-query
pub fn generate_ts_hooks(config: &Config) -> Result<String, Box<dyn std::error::Error>> {
    let routes = scan_controllers_folder(config)?;
    let ts_code = TypeScriptHooksGenerator::generate(&routes, &config.typescript)?;
    Ok(ts_code)
}
