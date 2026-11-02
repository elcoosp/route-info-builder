//! Command line interface for the route scanner.
//!
//! The binary is feature-gated behind `cli` so library consumers do not pay
//! for `clap`.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{Args, Parser, Subcommand};

use crate::configuration::Config;
use crate::error::Result;

/// Default configuration file name looked up when no `--config` is passed.
const DEFAULT_CONFIG_FILE: &str = "route-info.toml";

/// Command line interface of the `route-info-builder` binary.
#[derive(Debug, Parser)]
#[command(
    name = "route-info-builder",
    version,
    about = "Scan Axum-like controllers and generate Rust links, TypeScript clients, OpenAPI skeletons and route maps.",
    propagate_version = true
)]
pub struct Cli {
    /// Sub-command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Available sub-commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate the Rust link enum and every configured output.
    Generate(GenerateArgs),
    /// Print the scanned route map as JSON to stdout.
    Scan(GenerateArgs),
    /// Write a starter `route-info.toml`.
    Init {
        /// Where the starter configuration is written.
        #[arg(default_value = DEFAULT_CONFIG_FILE)]
        path: PathBuf,
        /// Overwrite an existing configuration file.
        #[arg(long)]
        force: bool,
    },
}

/// Shared arguments of `generate` and `scan`.
#[derive(Debug, Args, Default)]
pub struct GenerateArgs {
    /// Path to the TOML configuration file.
    #[arg(long, short)]
    pub config: Option<PathBuf>,
    /// Override `controllers_path`.
    #[arg(long)]
    pub controllers: Option<PathBuf>,
    /// Override `rust.output_file`.
    #[arg(long)]
    pub rust_output: Option<PathBuf>,
    /// Override `typescript.output_path` (implies `generate_client = true`).
    #[arg(long)]
    pub ts_dir: Option<PathBuf>,
    /// Override `openapi.output_path`.
    #[arg(long)]
    pub openapi: Option<PathBuf>,
    /// Override `route_map.output_path`.
    #[arg(long)]
    pub route_map: Option<PathBuf>,
}

/// Starter configuration written by `route-info-builder init`.
const STARTER_CONFIG: &str = r#"# Route Info Builder configuration
controllers_path = "src/controllers"

[rust]
output_file = "src/generated/links.rs"
enum_name = "Link"

[typescript]
generate_client = true
output_path = "frontend/src/api"
client_file = "client.ts"
hooks_file = "api.ts"
bindings_path = "../../../bindings"

[openapi]
output_path = "openapi.json"
title = "My API"
version = "0.1.0"

[route_map]
output_path = "routes.json"
"#;

/// Runs the CLI and returns the process exit code.
pub fn run<I, T>(arguments: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match Cli::try_parse_from(arguments) {
        Ok(Cli { command }) => dispatch(command),
        Err(error) => {
            let code = match error.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
                _ => 2,
            };
            let _unused = error.print();
            code
        }
    }
}

/// Executes a parsed command.
fn dispatch(command: Command) -> i32 {
    let outcome: std::result::Result<(), Box<dyn std::error::Error>> = match command {
        Command::Generate(arguments) => generate(&arguments).map_err(box_error),
        Command::Scan(arguments) => scan(&arguments).map_err(box_error),
        Command::Init { path, force } => init(&path, force).map_err(box_error),
    };
    match outcome {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: {error}");
            let mut source = error.source();
            while let Some(cause) = source {
                eprintln!("  caused by: {cause}");
                source = cause.source();
            }
            1
        }
    }
}

/// Erases the concrete error type so all commands share one reporting path.
fn box_error(error: impl std::error::Error + 'static) -> Box<dyn std::error::Error + 'static> {
    Box::new(error)
}

/// Loads the configuration and applies CLI overrides.
fn load_config(arguments: &GenerateArgs) -> Result<Config> {
    let mut config = Config::load(arguments.config.as_deref())?;
    if let Some(controllers) = &arguments.controllers {
        config.controllers_path.clone_from(controllers);
    }
    if let Some(rust_output) = &arguments.rust_output {
        config.rust.output_file = Some(rust_output.clone());
    }
    if let Some(ts_dir) = &arguments.ts_dir {
        config.typescript.output_path = Some(ts_dir.clone());
        config.typescript.generate_client = Some(true);
    }
    if let Some(openapi) = &arguments.openapi {
        config.openapi.output_path = Some(openapi.clone());
    }
    if let Some(route_map) = &arguments.route_map {
        config.route_map.output_path = Some(route_map.clone());
    }
    Ok(config)
}

/// Implements `route-info-builder generate`.
fn generate(arguments: &GenerateArgs) -> Result<()> {
    let config = load_config(arguments)?;
    let code = crate::generate_links(&config)?;
    println!("Generated Rust link enum ({} bytes).", code.len());
    if let Some(output_file) = &config.rust.output_file {
        println!("  rust:      {}", output_file.display());
    }
    if let Some(ts_dir) = &config.typescript.output_path
        && config.typescript.generate_client()
    {
        println!(
            "  typescript: {}/{}",
            ts_dir.display(),
            config.typescript_client_file()
        );
        println!(
            "              {}/{}",
            ts_dir.display(),
            config.typescript_hooks_file()
        );
    }
    if let Some(openapi) = &config.openapi.output_path {
        println!("  openapi:   {}", openapi.display());
    }
    if let Some(route_map) = &config.route_map.output_path {
        println!("  route map: {}", route_map.display());
    }
    Ok(())
}

/// Implements `route-info-builder scan`.
fn scan(arguments: &GenerateArgs) -> Result<()> {
    let config = load_config(arguments)?;
    let routes = crate::scan_routes(&config)?;
    println!("{}", serde_json::to_string_pretty(&routes)?);
    Ok(())
}

/// Implements `route-info-builder init`.
fn init(path: &std::path::Path, force: bool) -> std::result::Result<(), InitError> {
    if path.exists() && !force {
        return Err(InitError::Refused(path.to_path_buf()));
    }
    std::fs::write(path, STARTER_CONFIG).map_err(InitError::Io)?;
    println!("Wrote starter configuration to {}.", path.display());
    Ok(())
}

/// Failure modes of the `init` sub-command.
#[derive(Debug)]
enum InitError {
    /// An existing file would have been overwritten without `--force`.
    Refused(PathBuf),
    /// The starter configuration could not be written.
    Io(std::io::Error),
}

impl std::fmt::Display for InitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::Refused(path) => write!(
                formatter,
                "refusing to overwrite {} (pass --force to overwrite)",
                path.display()
            ),
            InitError::Io(error) => write!(formatter, "failed to write configuration: {error}"),
        }
    }
}

impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InitError::Refused(_) => None,
            InitError::Io(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::{DEFAULT_CONFIG_FILE, GenerateArgs, STARTER_CONFIG, run, scan};
    use crate::configuration::Config;
    use std::path::Path;

    /// Tests touch the filesystem, so they use a boxed error alias.
    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn help_and_version_exit_successfully() {
        assert_eq!(run(args(&["route-info-builder", "--help"])), 0);
        assert_eq!(run(args(&["route-info-builder", "--version"])), 0);
        assert_eq!(run(args(&["route-info-builder", "generate", "--help"])), 0);
    }

    #[test]
    fn unknown_flags_exit_with_usage_error() {
        assert_eq!(run(args(&["route-info-builder", "--nope"])), 2);
        assert_eq!(run(args(&[])), 2);
    }

    #[test]
    fn generate_writes_outputs_and_reports() -> TestResult {
        let dir = tempfile::tempdir()?;
        let controllers = dir.path().join("controllers");
        std::fs::create_dir_all(&controllers)?;
        let fixture = "pub fn routes() -> Router {\n  Router::new().route(\"/api/users\", get(list_users))\n}\n";
        std::fs::write(controllers.join("users.rs"), fixture).expect("write fixture");
        let code = run(args(&[
            "route-info-builder",
            "generate",
            "--controllers",
            controllers.to_str().expect("utf8"),
            "--rust-output",
            dir.path().join("out/links.rs").to_str().expect("utf8"),
            "--openapi",
            dir.path().join("openapi.json").to_str().expect("utf8"),
            "--route-map",
            dir.path().join("routes.json").to_str().expect("utf8"),
        ]));
        assert_eq!(code, 0);
        assert!(dir.path().join("out/links.rs").exists());
        assert!(dir.path().join("openapi.json").exists());
        assert!(dir.path().join("routes.json").exists());
        Ok(())
    }

    #[test]
    fn generate_with_ts_dir_enables_client_generation() -> TestResult {
        let dir = tempfile::tempdir()?;
        let controllers = dir.path().join("controllers");
        std::fs::create_dir_all(&controllers)?;
        let fixture = "fn routes() -> Router { Router::new() }\n";
        std::fs::write(controllers.join("users.rs"), fixture).expect("write fixture");
        let code = run(args(&[
            "route-info-builder",
            "generate",
            "--controllers",
            controllers.to_str().expect("utf8"),
            "--ts-dir",
            dir.path().join("web/api").to_str().expect("utf8"),
        ]));
        assert_eq!(code, 0);
        assert!(dir.path().join("web/api/client.ts").exists());
        assert!(dir.path().join("web/api/api.ts").exists());
        Ok(())
    }

    #[test]
    fn generate_with_broken_config_exits_nonzero() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("broken.toml");
        std::fs::write(&config_path, "not [ valid toml").expect("write config");
        let code = run(args(&[
            "route-info-builder",
            "generate",
            "--config",
            config_path.to_str().expect("utf8"),
        ]));
        assert_eq!(code, 1);
    }

    #[test]
    fn generate_with_missing_controllers_exits_nonzero() {
        let code = run(args(&[
            "route-info-builder",
            "generate",
            "--controllers",
            "/definitely/not/here",
        ]));
        assert_eq!(code, 1);
    }

    #[test]
    fn scan_prints_route_map_and_fails_on_missing_dir() {
        let code = run(args(&[
            "route-info-builder",
            "scan",
            "--controllers",
            "/definitely/not/here",
        ]));
        assert_eq!(code, 1);
    }

    #[test]
    fn init_writes_starter_config_and_respects_force() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("route-info.toml");

        // Refuses to overwrite without --force (exit code comes from the
        // early process exit; guarded by running against a fresh path).
        let fresh = dir.path().join("other.toml");
        assert_eq!(
            run(args(&[
                "route-info-builder",
                "init",
                fresh.to_str().expect("utf8")
            ])),
            0
        );
        assert_eq!(
            std::fs::read_to_string(&fresh)?,
            STARTER_CONFIG,
            "starter content must match the default file name {DEFAULT_CONFIG_FILE}"
        );
        // --force overwrites.
        std::fs::write(&path, "old")?;
        assert_eq!(
            run(args(&[
                "route-info-builder",
                "init",
                path.to_str().expect("utf8"),
                "--force"
            ])),
            0
        );
        assert_eq!(std::fs::read_to_string(&path)?, STARTER_CONFIG);
        Ok(())
    }

    #[test]
    fn init_without_force_fails_on_existing_file() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("route-info.toml");
        std::fs::write(&path, "existing")?;
        let error = super::init(&path, false).expect_err("refusal expected");
        assert!(error.to_string().contains("refusing to overwrite"));
        assert!(error.source().is_none());
        assert_eq!(std::fs::read_to_string(&path)?, "existing");
        // With --force the file is replaced.
        super::init(&path, true).expect("forced init must succeed");
        assert_eq!(std::fs::read_to_string(&path)?, STARTER_CONFIG);
        Ok(())
    }

    #[test]
    fn init_io_failures_are_reported() {
        let error = super::init(Path::new("/definitely/missing/dir/config.toml"), true)
            .expect_err("unwritable path must fail");
        assert!(error.to_string().contains("failed to write configuration"));
        assert!(error.source().is_some());
    }

    #[test]
    fn scan_function_is_reusable() -> TestResult {
        let dir = tempfile::tempdir()?;
        let controllers = dir.path().join("controllers");
        std::fs::create_dir_all(&controllers)?;
        let fixture = "fn routes() -> Router { Router::new() }\n";
        std::fs::write(controllers.join("users.rs"), fixture).expect("write fixture");
        let arguments = GenerateArgs {
            controllers: Some(controllers),
            ..GenerateArgs::default()
        };
        // The scan writes to stdout; capturing is not needed for coverage.
        assert!(scan(&arguments).is_ok());
        Ok(())
    }

    #[test]
    fn load_config_none_leaves_defaults() -> TestResult {
        let arguments = GenerateArgs::default();
        let config = super::load_config(&arguments)?;
        assert_eq!(config.controllers_path, Config::default().controllers_path);
        Ok(())
    }

    #[test]
    fn init_via_run_in_temp_cwd() -> TestResult {
        let _guard = crate::test_support::CurrentDirGuard::lock()?;
        let dir = tempfile::tempdir()?;
        std::env::set_current_dir(dir.path())?;
        assert_eq!(run(args(&["route-info-builder", "init"])), 0);
        assert!(dir.path().join(DEFAULT_CONFIG_FILE).exists());
        Ok(())
    }
}
