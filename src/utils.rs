//! Small string and path helpers shared by the parser and the generators.

pub(crate) mod case;
pub(crate) mod path;

/// Formatting trait, only needed by the test helper below.
#[cfg(test)]
use std::fmt::Write as _;

/// Emits a `cargo:warning` line so build-script users see the message.
///
/// When the crate is used outside a build script the line is simply printed
/// to stderr.
pub(crate) fn build_warning(message: &str) {
    eprintln!("cargo:warning={message}");
}

/// Appends `message` to `out` using a `cargo:warning=` prefix (testable variant).
#[cfg(test)]
pub(crate) fn warning_string(out: &mut String, message: &str) {
    let _ = write!(out, "cargo:warning={message}");
}

#[cfg(test)]
mod tests {
    use super::{build_warning, warning_string};

    #[test]
    fn build_warning_prints_without_panic() {
        // Coverage: exercise the stderr path; the message goes to the captured
        // test stderr and cannot break the build.
        build_warning("route-info-builder: test warning");
    }

    #[test]
    fn warning_string_formats_prefix() {
        let mut out = String::new();
        warning_string(&mut out, "hello");
        assert_eq!(out, "cargo:warning=hello");
    }
}
