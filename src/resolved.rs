//! One-pass route name resolution shared by every generator.
//!
//! Scan time assigns each [`RouteInfo`] a *base* name derived from its path
//! and HTTP method. Every generated artifact must agree on the final names:
//! the Rust enum variant, the TypeScript client key, the `TanStack` Query
//! hook name and the `OpenAPI` `operationId` all refer to the same route, and the
//! crate documents the Rust `name()` / TypeScript-key parity as a guarantee.
//!
//! Before this module existed each generator re-derived its display names,
//! so the collision strategy and the variant prefix/suffix decorations were
//! applied by the Rust generator only. The TypeScript artifacts could then
//! emit duplicate object keys or function names (a compile error in the
//! generated code) and `operationId`s could repeat. Resolution now happens
//! exactly once, here, and every generator consumes the resolved pair.

use crate::configuration::NamingConfig;
use crate::configuration::naming::{VariantNamer, variant_base_name};
use crate::parser::RouteInfo;
use crate::utils::build_warning;
use crate::utils::case;

/// A scanned route paired with its final, collision-resolved names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedRoute<'a> {
    /// The scanned route (path, method, handler metadata, ...).
    pub route: &'a RouteInfo,
    /// Final variant identifier body used by the generated Rust enum
    /// (case-converted base name plus the configured prefix/suffix).
    pub variant: String,
    /// camelCase of [`NamedRoute::variant`]; used as the TypeScript client
    /// key, the `TanStack` Query hook name suffix and the `OpenAPI`
    /// `operationId`. Equal to the value returned by the generated Rust
    /// `name()` method, which keeps the two worlds in parity.
    pub ts_name: String,
}

impl NamedRoute<'_> {
    /// Name of the TypeScript interface carrying the path parameters of
    /// this route (`GetUsersParams`, ...).
    #[must_use]
    pub fn params_interface(&self) -> String {
        format!("{}Params", case::convert_to_case(&self.ts_name, "pascal"))
    }
}

/// Resolves the final names for every route, preserving the input order.
///
/// The [`NamingConfig`] decorations (`variant_case`, `variant_prefix`,
/// `variant_suffix`, `preserve_numbers`) and the configured
/// [`CollisionStrategy`](crate::CollisionStrategy) are applied exactly once:
/// routes whose variant name collides under the `skip` strategy are dropped
/// with a `cargo:warning` instead of reaching any generator.
///
/// Routes dropped here are the same ones the generated Rust enum never
/// contained; the remaining routes are guaranteed unique, collision-resolved
/// names across all artifacts.
#[must_use]
pub fn resolve_route_names<'a>(
    routes: &'a [RouteInfo],
    naming: &NamingConfig,
) -> Vec<NamedRoute<'a>> {
    let mut namer = VariantNamer::new(naming.collision_strategy());
    let mut resolved = Vec::with_capacity(routes.len());
    for route in routes {
        let base = variant_base_name(&route.name, naming);
        let Some(variant) = namer.assign(&base, &route.method) else {
            build_warning(&format!(
                "route-info-builder: duplicate variant name `{base}` skipped for route {} {}",
                route.method, route.path
            ));
            continue;
        };
        let ts_name = case::convert_to_case(&variant, "camel");
        resolved.push(NamedRoute {
            route,
            variant,
            ts_name,
        });
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::{NamedRoute, resolve_route_names};
    use crate::configuration::{CollisionStrategy, NamingConfig};
    use crate::parser::{HandlerInfo, RouteInfo};

    fn route(name: &str, path: &str, method: &str) -> RouteInfo {
        RouteInfo {
            name: name.to_string(),
            path: path.to_string(),
            method: method.to_string(),
            handler: format!("{name}_handler"),
            handler_info: HandlerInfo::default(),
        }
    }

    #[test]
    fn distinct_routes_resolve_in_order() {
        let routes = vec![
            route("get_users", "/users", "GET"),
            route("post_users", "/users", "POST"),
            route("get_users_user_id", "/users/{user_id}", "GET"),
        ];
        let named = resolve_route_names(&routes, &NamingConfig::default());
        assert_eq!(named.len(), 3);
        assert_eq!(named[0].variant, "GetUsers");
        assert_eq!(named[0].ts_name, "getUsers");
        assert_eq!(named[1].variant, "PostUsers");
        assert_eq!(named[1].ts_name, "postUsers");
        assert_eq!(named[2].variant, "GetUsersUserId");
        assert_eq!(named[2].ts_name, "getUsersUserId");
        // The resolved entry keeps pointing at the original route.
        assert_eq!(named[0].route.path, "/users");
        assert_eq!(named[0].route.method, "GET");
    }

    #[test]
    fn ts_name_is_always_the_camel_case_of_the_variant() {
        // Parity property: whatever the decorations, the TypeScript-facing
        // name is derived from the final variant, never from the raw base.
        let naming = NamingConfig {
            variant_prefix: Some("Api".to_string()),
            variant_suffix: Some("Route".to_string()),
            ..NamingConfig::default()
        };
        let routes = vec![route("get_users", "/users", "GET")];
        let named = resolve_route_names(&routes, &naming);
        assert_eq!(named[0].variant, "ApiGetUsersRoute");
        assert_eq!(named[0].ts_name, "apiGetUsersRoute");
    }

    #[test]
    fn skip_strategy_drops_colliding_routes() {
        let routes = vec![
            route("users", "/users", "GET"),
            route("users", "/users", "POST"),
            route("users", "/users", "PUT"),
        ];
        let named = resolve_route_names(&routes, &NamingConfig::default());
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].route.method, "GET");
    }

    #[test]
    fn append_method_strategy_resolves_collisions() {
        let naming = NamingConfig {
            collision_strategy: Some(CollisionStrategy::AppendMethod),
            ..NamingConfig::default()
        };
        let routes = vec![
            route("users", "/users", "GET"),
            route("users", "/users", "POST"),
        ];
        let named = resolve_route_names(&routes, &naming);
        assert_eq!(named.len(), 2);
        assert_eq!(named[0].variant, "Users");
        assert_eq!(named[0].ts_name, "users");
        assert_eq!(named[1].variant, "PostUsers");
        assert_eq!(named[1].ts_name, "postUsers");
    }

    #[test]
    fn number_strategy_resolves_collisions() {
        let naming = NamingConfig {
            collision_strategy: Some(CollisionStrategy::Number),
            ..NamingConfig::default()
        };
        let routes = vec![
            route("users", "/users", "GET"),
            route("users", "/users", "POST"),
            route("users", "/users", "PATCH"),
        ];
        let named = resolve_route_names(&routes, &naming);
        let variants: Vec<&str> = named.iter().map(|entry| entry.variant.as_str()).collect();
        assert_eq!(variants, vec!["Users", "Users2", "Users3"]);
    }

    #[test]
    fn params_interface_is_derived_from_the_resolved_name() {
        let naming = NamingConfig {
            variant_prefix: Some("Api".to_string()),
            ..NamingConfig::default()
        };
        let routes = vec![route("get_users_user_id", "/users/{user_id}", "GET")];
        let named = resolve_route_names(&routes, &naming);
        assert_eq!(named[0].params_interface(), "ApiGetUsersUserIdParams");
    }

    #[test]
    fn empty_input_yields_no_named_routes() {
        let named = resolve_route_names(&[], &NamingConfig::default());
        assert_eq!(named, Vec::<NamedRoute<'_>>::new());
    }

    #[test]
    fn named_route_is_debuggable_and_comparable() {
        let routes = vec![route("get_users", "/users", "GET")];
        let named = resolve_route_names(&routes, &NamingConfig::default());
        let first: NamedRoute<'_> = named[0].clone();
        assert!(format!("{first:?}").contains("NamedRoute"));
        assert_eq!(first, named[0]);
    }
}
