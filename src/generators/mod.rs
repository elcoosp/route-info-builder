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

pub trait RouteProcessor {
    fn process_routes(routes: &[RouteInfo]) -> Result<ProcessedRoutes, Box<dyn std::error::Error>>;
}

#[derive(Debug, Clone)]
pub struct ProcessedRoutes {
    pub routes: Vec<RouteInfo>,
    pub grouped_by_method: std::collections::HashMap<String, Vec<RouteInfo>>,
    pub unique_paths: std::collections::HashSet<String>,
}
