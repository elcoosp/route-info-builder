//! Code generators turning scanned routes into Rust, TypeScript and `OpenAPI`
//! artifacts.

pub(crate) mod openapi;
pub(crate) mod rust;
pub(crate) mod typescript;

use crate::error::Result;
use crate::parser::RouteInfo;

/// Common interface implemented by all generators.
pub trait CodeGenerator {
    /// Configuration type consumed by this generator.
    type Config;
    /// Artifact produced by this generator.
    type Output;

    /// Generates the artifact for the given routes.
    ///
    /// # Errors
    ///
    /// Implementation-specific; see the implementing generator.
    fn generate(routes: &[RouteInfo], config: &Self::Config) -> Result<Self::Output>;
}
