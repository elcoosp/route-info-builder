//! Typed errors produced by the route scanning and code generation pipeline.

use std::path::PathBuf;

/// Convenient result alias used across the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// All errors that [`scan`](crate::scan_routes) and the generators can emit.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The controllers directory (or one of its sub-directories) could not be read.
    #[error("failed to read controllers directory `{path}`: {source}")]
    ReadDir {
        /// Directory that could not be listed.
        path: PathBuf,
        /// Underlying I/O failure.
        #[source]
        source: std::io::Error,
    },

    /// A controller file could not be read.
    #[error("failed to read controller file `{path}`: {source}")]
    ReadFile {
        /// File that could not be read.
        path: PathBuf,
        /// Underlying I/O failure.
        #[source]
        source: std::io::Error,
    },

    /// An output directory could not be created.
    #[error("failed to create output directory `{path}`: {source}")]
    CreateDir {
        /// Directory that could not be created.
        path: PathBuf,
        /// Underlying I/O failure.
        #[source]
        source: std::io::Error,
    },

    /// An output file could not be written.
    #[error("failed to write output file `{path}`: {source}")]
    WriteFile {
        /// File that could not be written.
        path: PathBuf,
        /// Underlying I/O failure.
        #[source]
        source: std::io::Error,
    },

    /// A controller file is not valid Rust source.
    #[error("failed to parse controller file `{path}`: {source}")]
    Parse {
        /// File that failed to parse.
        path: PathBuf,
        /// Underlying syntax error reported by `syn`.
        #[source]
        source: syn::Error,
    },

    /// The configuration file is not valid TOML or contains unknown keys.
    #[error("invalid configuration: {0}")]
    Config(#[from] toml::de::Error),

    /// Serializing the route map or the `OpenAPI` document failed.
    #[error("failed to serialize generated data: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::Error;

    #[test]
    fn read_dir_display_and_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err = Error::ReadDir {
            path: "/controllers".into(),
            source: io_err,
        };
        assert_eq!(
            err.to_string(),
            "failed to read controllers directory `/controllers`: missing"
        );
        assert!(err.source().is_some());
    }

    #[test]
    fn read_file_display_and_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = Error::ReadFile {
            path: "/c/users.rs".into(),
            source: io_err,
        };
        assert_eq!(
            err.to_string(),
            "failed to read controller file `/c/users.rs`: denied"
        );
        assert!(err.source().is_some());
    }

    #[test]
    fn create_dir_display_and_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::AlreadyExists, "exists");
        let err = Error::CreateDir {
            path: "/out".into(),
            source: io_err,
        };
        assert_eq!(
            err.to_string(),
            "failed to create output directory `/out`: exists"
        );
        assert!(err.source().is_some());
    }

    #[test]
    fn write_file_display_and_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = Error::WriteFile {
            path: "/out/links.rs".into(),
            source: io_err,
        };
        assert_eq!(
            err.to_string(),
            "failed to write output file `/out/links.rs`: denied"
        );
        assert!(err.source().is_some());
    }

    #[test]
    fn parse_display_and_source() {
        let syn_err = syn::Error::new(proc_macro2::Span::call_site(), "bad syntax");
        let err = Error::Parse {
            path: "/c/bad.rs".into(),
            source: syn_err,
        };
        let message = err.to_string();
        assert!(message.starts_with("failed to parse controller file `/c/bad.rs`: "));
        assert!(message.contains("bad syntax"));
        assert!(err.source().is_some());
    }

    #[test]
    fn config_display() {
        let err: Error = toml::from_str::<toml::Value>("!!! not toml")
            .map_err(Error::Config)
            .expect_err("invalid toml must fail");
        assert!(err.to_string().starts_with("invalid configuration:"));
    }

    #[test]
    fn json_display_and_source() {
        let json_err =
            serde_json::from_str::<serde_json::Value>("!!!").expect_err("invalid json must fail");
        let err = Error::Json(json_err);
        assert!(
            err.to_string()
                .starts_with("failed to serialize generated data: ")
        );
        assert!(err.source().is_some());
    }
}
