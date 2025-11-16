pub mod rust;
pub mod typescript;

use crate::RouteInfo;

pub trait CodeGenerator {
    type Config;
    type Output;

    fn generate(
        routes: &[RouteInfo],
        config: &Self::Config,
    ) -> Result<Self::Output, Box<dyn std::error::Error>>;
}
