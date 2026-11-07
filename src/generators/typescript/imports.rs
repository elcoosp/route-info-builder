//! Shared bookkeeping for TypeScript type imports collected from routes.

use std::collections::BTreeSet;

use crate::parser::RouteInfo;

/// Collects the user types referenced by routes and renders their import
/// statements from the configured bindings directory.
#[derive(Debug, Default)]
pub(crate) struct TypeImportManager {
    type_imports: BTreeSet<String>,
    error_imports: BTreeSet<String>,
}

impl TypeImportManager {
    /// Collects body, return and error types from all routes.
    pub(crate) fn collect_from_routes<'a, I>(&mut self, routes: I)
    where
        I: IntoIterator<Item = &'a RouteInfo>,
    {
        for route in routes {
            if let Some(body_type) = &route.handler_info.body_param {
                self.collect_type(body_type);
            }
            if let Some(query_type) = &route.handler_info.query_params {
                self.collect_type(query_type);
            }
            if let Some(return_type) = &route.handler_info.return_type.found_type
                && route.handler_info.return_type.is_importable
            {
                self.collect_type(return_type);
            }
            for error_type in &route.handler_info.return_type.error_types {
                self.error_imports.insert(error_type.clone());
            }
        }
    }

    /// Registers a type, unwrapping generics such as `Array<T>`,
    /// `Option<T>` and `Result<T, E>` recursively.
    pub(crate) fn collect_type(&mut self, type_str: &str) {
        if let Some(inner) = Self::generic_inner_type(type_str) {
            self.collect_type(&inner);
        } else if !Self::is_builtin_type(type_str) {
            self.type_imports.insert(type_str.to_string());
        }
    }

    /// Inner type name of simple generic wrappers.
    fn generic_inner_type(type_str: &str) -> Option<String> {
        let unwrap_suffix = |prefix: &str| -> Option<String> {
            let inner = type_str.strip_prefix(prefix)?;
            let inner = inner.strip_suffix('>')?;
            Some(inner.trim().to_string())
        };
        if let Some(inner) = unwrap_suffix("Array<") {
            return Some(inner);
        }
        if let Some(inner) = unwrap_suffix("Option<") {
            return Some(inner);
        }
        // `Result<T, E>`: only the success type is relevant.
        unwrap_suffix("Result<").map(|inner| {
            inner
                .split(',')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        })
    }

    /// Built-in TypeScript types that must not be imported.
    fn is_builtin_type(name: &str) -> bool {
        matches!(
            name,
            "string"
                | "number"
                | "boolean"
                | "any"
                | "void"
                | "unknown"
                | "null"
                | "undefined"
                | "Array"
                | "Promise"
        )
    }

    /// Sorted, deterministic import statements for all collected types.
    pub(crate) fn generate_imports(&self, bindings_path: &str) -> Vec<String> {
        self.type_imports
            .union(&self.error_imports)
            .map(|type_name| {
                format!("import {{ type {type_name} }} from \"{bindings_path}/{type_name}\";")
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::TypeImportManager;
    use crate::parser::{HandlerInfo, RouteInfo, handlers::ReturnType};
    use std::collections::BTreeSet;

    fn route(
        body: Option<&str>,
        found: Option<&str>,
        importable: bool,
        errors: &[&str],
    ) -> RouteInfo {
        RouteInfo {
            name: "get_x".to_string(),
            path: "/x".to_string(),
            method: "GET".to_string(),
            handler: "get_x_handler".to_string(),
            handler_info: HandlerInfo {
                body_param: body.map(str::to_string),
                query_params: None,
                requires_auth: false,
                return_type: ReturnType {
                    found_type: found.map(str::to_string),
                    is_importable: importable,
                    error_types: errors.iter().map(|error| (*error).to_string()).collect(),
                    error_variants: BTreeSet::new(),
                },
            },
        }
    }

    #[test]
    fn collects_unwrapping_generics_and_skips_builtins() {
        let mut manager = TypeImportManager::default();
        manager.collect_type("Array<UserResponse>");
        manager.collect_type("Option<Nested>");
        manager.collect_type("Result<Deep, Error>");
        manager.collect_type("string");
        manager.collect_type("Array");
        manager.collect_type("Promise");
        manager.collect_type("Plain");
        manager.collect_type("Array<  Spaced  >");
        let imports = manager.generate_imports("../bindings");
        assert_eq!(
            imports,
            vec![
                "import { type Deep } from \"../bindings/Deep\";",
                "import { type Nested } from \"../bindings/Nested\";",
                "import { type Plain } from \"../bindings/Plain\";",
                "import { type Spaced } from \"../bindings/Spaced\";",
                "import { type UserResponse } from \"../bindings/UserResponse\";",
            ]
        );
    }

    #[test]
    fn error_imports_are_collected_from_routes() {
        let routes = vec![
            route(Some("CreateUserRequest"), None, false, &["BadRequest"]),
            route(None, Some("UserResponse"), true, &[]),
            route(None, Some("any"), true, &[]),
        ];
        let mut manager = TypeImportManager::default();
        manager.collect_from_routes(&routes);
        let imports = manager.generate_imports("../../../bindings");
        assert_eq!(
            imports,
            vec![
                "import { type BadRequest } from \"../../../bindings/BadRequest\";",
                "import { type CreateUserRequest } from \"../../../bindings/CreateUserRequest\";",
                "import { type UserResponse } from \"../../../bindings/UserResponse\";",
            ]
        );
    }

    #[test]
    fn non_importable_return_types_are_ignored() {
        let routes = vec![route(None, Some("UserResponse"), false, &[])];
        let mut manager = TypeImportManager::default();
        manager.collect_from_routes(&routes);
        assert_eq!(manager.generate_imports("b"), Vec::<String>::new());
    }

    #[test]
    fn debug_is_available() {
        let manager = TypeImportManager::default();
        assert!(format!("{manager:?}").contains("TypeImportManager"));
    }
}
