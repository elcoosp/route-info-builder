//! TypeScript code generation: typed client, `TanStack` Query hooks and the
//! shared type-import bookkeeping.

pub(crate) mod client;
pub(crate) mod hooks;
pub(crate) mod imports;

use std::fmt::Write as _;

use crate::parser::RouteInfo;
use crate::utils::case;

pub(crate) use imports::TypeImportManager;

pub use client::ClientGenerator;
pub use hooks::HooksGenerator;

/// camelCase method name used both in the client object and as hook query key.
#[must_use]
pub(crate) fn method_display_name(route: &RouteInfo) -> String {
    case::convert_to_case(&route.name, "camel")
}

/// Name of the generated `...Params` interface for a route.
#[must_use]
pub(crate) fn params_interface_name(route: &RouteInfo) -> String {
    format!(
        "{}Params",
        case::convert_to_case(&method_display_name(route), "pascal")
    )
}

/// Converts a Rust-side parameter name to its camelCase TS property name.
#[must_use]
pub(crate) fn ts_param_name(param: &str) -> String {
    case::convert_to_case(param, "camel")
}

/// Builds the TypeScript template literal for a normalized route path.
#[must_use]
pub(crate) fn path_template(route_path: &str) -> String {
    let mut template = String::new();
    for segment in route_path.split('/').filter(|segment| !segment.is_empty()) {
        template.push('/');
        match crate::utils::path::param_name(segment) {
            Some(param) => {
                let _unused = write!(template, "${{params.{}}}", ts_param_name(param));
            }
            None => template.push_str(segment),
        }
    }
    if template.is_empty() {
        template.push('/');
    }
    format!("`{template}`")
}

/// Pascal-cases an input, used for TS type names.
#[must_use]
pub(crate) fn pascal(input: &str) -> String {
    case::convert_to_case(input, "pascal")
}

#[cfg(test)]
mod tests {
    use super::{method_display_name, params_interface_name, pascal, path_template};
    use crate::parser::{HandlerInfo, RouteInfo};

    fn route(name: &str, path: &str) -> RouteInfo {
        RouteInfo {
            name: name.to_string(),
            path: path.to_string(),
            method: "GET".to_string(),
            handler: "handler".to_string(),
            handler_info: HandlerInfo::default(),
        }
    }

    #[test]
    fn display_names_are_camel_cased() {
        assert_eq!(method_display_name(&route("get_user", "/u")), "getUser");
        assert_eq!(
            params_interface_name(&route("get_user", "/u")),
            "GetUserParams"
        );
    }

    #[test]
    fn path_template_handles_params_and_root() {
        assert_eq!(
            path_template("/api/users/{user_id}"),
            "`/api/users/${params.userId}`"
        );
        assert_eq!(path_template("/files/{*rest}"), "`/files/${params.rest}`");
        assert_eq!(path_template("/"), "`/`");
        assert_eq!(path_template("/health"), "`/health`");
    }

    #[test]
    fn case_helpers() {
        assert_eq!(pascal("get_user"), "GetUser");
    }
}
