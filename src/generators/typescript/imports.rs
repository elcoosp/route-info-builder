use ts_quote::ts_string;

// [file name]: imports.rs
use crate::RouteInfo;
use std::collections::HashSet;

/// Shared utilities for handling TypeScript type imports
pub struct TypeImportManager {
    pub type_imports: HashSet<String>,
    pub error_imports: HashSet<String>,
}

impl TypeImportManager {
    pub fn new() -> Self {
        Self {
            type_imports: HashSet::new(),
            error_imports: HashSet::new(),
        }
    }

    /// Collect all importable types from routes
    pub fn collect_from_routes(&mut self, routes: &[RouteInfo]) {
        for route in routes {
            // Handle body types
            if let Some(body_type) = &route.handler_info.body_param {
                self.extract_importable_types(body_type);
            }

            // Handle return types
            if let Some(return_type) = &route.handler_info.return_type.found_type
                && route.handler_info.return_type.is_importable
            {
                self.extract_importable_types(return_type);
            }

            // Handle error types
            for error_type in &route.handler_info.return_type.error_types {
                self.error_imports.insert(error_type.clone());
            }
        }
    }

    /// Extract importable types from type strings, handling generics like Array<T>
    pub fn extract_importable_types(&mut self, type_str: &str) {
        // Check if this is a generic type like Array<T>
        if let Some(inner_type) = Self::extract_generic_inner_type(type_str) {
            // Recursively extract inner types (for nested generics)
            self.extract_importable_types(&inner_type);
        } else if !Self::is_builtin_type(type_str) {
            // Only add non-builtin types
            self.type_imports.insert(type_str.to_string());
        }
    }

    /// Extract the inner type from generic types like Array<T>, Option<T>, etc.
    fn extract_generic_inner_type(type_str: &str) -> Option<String> {
        if type_str.starts_with("Array<") && type_str.ends_with('>') {
            Some(type_str[6..type_str.len() - 1].to_string())
        } else if type_str.starts_with("Option<") && type_str.ends_with('>') {
            Some(type_str[7..type_str.len() - 1].to_string())
        } else if type_str.starts_with("Result<") && type_str.ends_with('>') {
            // For Result<T, E>, we only care about the success type T
            let inner = &type_str[7..type_str.len() - 1];
            inner.split(',').next().map(|s| s.trim().to_string())
        } else {
            None
        }
    }

    /// Check if a type is a built-in TypeScript type that shouldn't be imported
    fn is_builtin_type(type_name: &str) -> bool {
        type_name == "string" ||
        type_name == "number" ||
        type_name == "boolean" ||
        type_name == "any" ||
        type_name == "void" ||
        type_name == "unknown" ||
        type_name == "null" ||
        type_name == "undefined" ||
        type_name == "Array" ||  // Array without generic is built-in
        type_name == "Promise" // Promise is built-in
    }

    /// Generate import statements for collected types
    pub fn generate_imports(&self) -> Vec<String> {
        let mut imports = Vec::new();

        // Generate type imports
        for type_name in &self.type_imports {
            // FIXME get path from config
            let import_path = format!("\"../../../bindings/{type_name}\"");
            imports.push(
                ts_string! {
                    import { type #type_name } from #import_path;
                }
                .to_string(),
            );
        }

        // Generate error type imports
        for type_name in &self.error_imports {
            let import_path = format!("\"../../../bindings/{type_name}\"");
            imports.push(
                ts_string! {
                    import { type #type_name } from #import_path;
                }
                .to_string(),
            );
        }

        imports
    }
}

impl Default for TypeImportManager {
    fn default() -> Self {
        Self::new()
    }
}
