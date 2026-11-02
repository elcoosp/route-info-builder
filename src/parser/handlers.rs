//! Extracts per-handler information (body/query extractors, auth, return and
//! error types) from the functions of a controller file.
//!
//! The original implementation hand-rolled its AST recursion; it now uses
//! `syn`'s [`Visit`] infrastructure, which also covers closures, `try` blocks
//! and nested statements automatically.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::Serialize;
use syn::visit::{Visit, visit_expr_call, visit_expr_return};

use crate::parser::HandlerInfo;

/// Return type information recovered from a handler body.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnType {
    /// Type detected in the first `format::json(...)` conversion, if any.
    pub found_type: Option<String>,
    /// Whether `found_type` is a user type worth importing.
    pub is_importable: bool,
    /// Error enum names returned by the handler.
    pub error_types: BTreeSet<String>,
    /// Full error variant paths returned by the handler.
    pub error_variants: BTreeSet<String>,
}

/// Collects the JSON conversion type from the first `json(...)` call.
struct ReturnTypeVisitor {
    found: Option<(String, bool)>,
}

impl Visit<'_> for ReturnTypeVisitor {
    fn visit_expr_call(&mut self, call: &syn::ExprCall) {
        if self.found.is_some() {
            return;
        }
        if let syn::Expr::Path(func_path) = &*call.func
            && let Some(segment) = func_path.path.segments.last()
            && segment.ident == "json"
            && let Some(first_arg) = call.args.first()
        {
            self.analyze_conversion(first_arg);
            return;
        }
        visit_expr_call(self, call);
    }
}

impl ReturnTypeVisitor {
    /// Interprets `arg` of a `json(arg)` call as a conversion expression.
    fn analyze_conversion(&mut self, expr: &syn::Expr) {
        if self.found.is_some() {
            return;
        }
        let syn::Expr::Call(call) = expr else {
            return;
        };
        let syn::Expr::Path(func_path) = &*call.func else {
            return;
        };
        let path = &func_path.path;

        // `Vec::<T>::new()` converts to `Array<T>`.
        if let Some((ts_type, importable)) = vec_conversion(path) {
            self.found = Some((ts_type, importable));
            return;
        }
        // `Type::from(...)` keeps the type name.
        if path.segments.len() >= 2
            && let Some(type_segment) = path.segments.iter().nth_back(1)
        {
            self.found = Some((type_segment.ident.to_string(), true));
            return;
        }
        // `Type(...)` constructor call.
        if let Some(type_segment) = path.segments.last() {
            self.found = Some((type_segment.ident.to_string(), true));
        }
    }
}

/// Maps a `Vec::<T>::new()`-style path to `Array<T>` plus importability.
fn vec_conversion(path: &syn::Path) -> Option<(String, bool)> {
    let first = path.segments.first()?;
    if first.ident != "Vec" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(generics) = &first.arguments else {
        return None;
    };
    let Some(syn::GenericArgument::Type(syn::Type::Path(type_path))) = generics.args.first() else {
        return None;
    };
    let inner = type_path.path.segments.last()?.ident.to_string();
    let importable = !is_builtin_rust_type(&inner);
    Some((format!("Array<{inner}>"), importable))
}

/// Rust built-in types that do not need to be imported.
fn is_builtin_rust_type(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "str"
            | "String"
    )
}

/// Collects error types and variants from `Err(...)` returns.
struct ErrorCollector {
    error_types: BTreeSet<String>,
    error_variants: BTreeSet<String>,
}

impl Visit<'_> for ErrorCollector {
    fn visit_expr_call(&mut self, call: &syn::ExprCall) {
        if let syn::Expr::Path(func_path) = &*call.func
            && let Some(segment) = func_path.path.segments.last()
            && segment.ident == "Err"
            && let Some(first_arg) = call.args.first()
        {
            self.collect(first_arg);
        }
        visit_expr_call(self, call);
    }

    fn visit_expr_return(&mut self, expr_return: &syn::ExprReturn) {
        if let Some(returned) = &expr_return.expr {
            self.collect(returned);
        }
        visit_expr_return(self, expr_return);
    }
}

impl ErrorCollector {
    /// Recursively inspects `Err(x.into())`, paths and struct literals.
    fn collect(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::MethodCall(call) => {
                if call.method == "into" {
                    self.collect(&call.receiver);
                }
            }
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(func_path) = &*call.func
                    && let Some(segment) = func_path.path.segments.last()
                    && segment.ident == "Err"
                    && let Some(first_arg) = call.args.first()
                {
                    self.collect(first_arg);
                }
            }
            syn::Expr::Path(path_expr) => self.add_path(&path_expr.path),
            syn::Expr::Struct(struct_expr) => {
                let full_path = struct_expr
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                self.error_variants.insert(full_path.clone());
                if let Some(last) = struct_expr.path.segments.last() {
                    self.error_types.insert(last.ident.to_string());
                }
            }
            _ => {}
        }
    }

    /// Records a variant path and its parent error enum.
    fn add_path(&mut self, path: &syn::Path) {
        let full_path = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        // Skip plain lower-case variables: they are not type names.
        if !is_type_like(&full_path) {
            return;
        }
        self.error_variants.insert(full_path);
        if path.segments.len() > 1 {
            if let Some(parent) = path.segments.iter().nth_back(1) {
                self.error_types.insert(parent.ident.to_string());
            }
        } else if let Some(last) = path.segments.last() {
            self.error_types.insert(last.ident.to_string());
        }
    }
}

/// Heuristic: the first path segment must look like a type name.
fn is_type_like(path: &str) -> bool {
    path.split("::")
        .next()
        .is_some_and(|first| first.chars().next().is_some_and(char::is_uppercase))
}

/// Extracts body/query parameter types, auth usage and return/error types for
/// every function in the file, keyed by function name.
#[must_use]
pub(crate) fn extract_handler_info(file: &syn::File) -> BTreeMap<String, HandlerInfo> {
    let mut info = BTreeMap::new();
    for item in &file.items {
        let syn::Item::Fn(func) = item else {
            continue;
        };
        let mut body_param = None;
        let mut query_params = None;
        let mut requires_auth = false;

        for input in &func.sig.inputs {
            let syn::FnArg::Typed(pat_type) = input else {
                continue;
            };
            let syn::Type::Path(type_path) = &*pat_type.ty else {
                continue;
            };
            // A parsed `Type::Path` always has segments, but hand-built ASTs
            // (proc-macro consumers) may not; treat them as plain parameters.
            let Some(segment) = type_path.path.segments.last() else {
                continue;
            };
            let type_ident = segment.ident.to_string();
            let inner = first_generic_type_name(segment);
            match type_ident.as_str() {
                "Json" | "JsonValidate" | "JsonValidateWithMessage" => {
                    if let Some(inner) = inner {
                        body_param = Some(inner);
                    }
                }
                "Query" => {
                    if let Some(inner) = inner {
                        query_params = Some(inner);
                    }
                }
                _ => {}
            }
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat
                && pat_ident.ident == "auth"
                && type_ident == "JWT"
            {
                requires_auth = true;
            }
        }

        let mut return_visitor = ReturnTypeVisitor { found: None };
        return_visitor.visit_block(&func.block);
        let mut error_collector = ErrorCollector {
            error_types: BTreeSet::new(),
            error_variants: BTreeSet::new(),
        };
        error_collector.visit_block(&func.block);

        info.insert(
            func.sig.ident.to_string(),
            HandlerInfo {
                body_param,
                query_params,
                requires_auth,
                return_type: ReturnType {
                    found_type: return_visitor.found.as_ref().map(|(name, _)| name.clone()),
                    is_importable: return_visitor
                        .found
                        .is_some_and(|(_, importable)| importable),
                    error_types: error_collector.error_types,
                    error_variants: error_collector.error_variants,
                },
            },
        );
    }
    info
}

/// Returns the last segment name of the first generic argument, e.g. `T` for
/// `Json<T>` (only simple paths are considered).
fn first_generic_type_name(segment: &syn::PathSegment) -> Option<String> {
    let syn::PathArguments::AngleBracketed(generics) = &segment.arguments else {
        return None;
    };
    let Some(syn::GenericArgument::Type(syn::Type::Path(param_type))) = generics.args.first()
    else {
        return None;
    };
    param_type
        .path
        .segments
        .last()
        .map(|last| last.ident.to_string())
}

#[cfg(test)]
mod tests {
    use super::{ReturnTypeVisitor, extract_handler_info, is_type_like, vec_conversion};
    use syn::visit::Visit;

    fn parse_file(source: &str) -> syn::File {
        syn::parse_file(source).expect("fixture must parse")
    }

    #[test]
    fn extracts_body_query_auth_and_return_types() {
        let file = parse_file(
            r"
            fn list_users(auth: JWT, query: Query<ListUsersQuery>) -> Response {
                format::json(Vec::<UserResponse>::new())
            }
            fn get_user(auth: JWT) -> Response {
                format::json(UserResponse::from(user))
            }
            fn create_user(req: JsonValidate<CreateUserRequest>) -> Response {
                Err(BadRequest::EmailAlreadyExists.into())
            }
            fn with_message(body: JsonValidateWithMessage<Payload>) -> Response {
                format::json(HealthResponse(false))
            }
            fn plain(body: Json<Plain>) -> Response {
                format::json(other_value)
            }
            fn unknown_return() -> Response {
                let value = build();
                value
            }
            fn no_extractors() -> Response {
                format::json(anything)
            }
            ",
        );
        let info = extract_handler_info(&file);

        let list_users = &info["list_users"];
        assert_eq!(list_users.query_params.as_deref(), Some("ListUsersQuery"));
        assert!(list_users.requires_auth);
        assert_eq!(
            list_users.return_type.found_type.as_deref(),
            Some("Array<UserResponse>")
        );
        assert!(list_users.return_type.is_importable);

        let get_user = &info["get_user"];
        assert_eq!(
            get_user.return_type.found_type.as_deref(),
            Some("UserResponse")
        );
        assert!(get_user.return_type.is_importable);

        let create_user = &info["create_user"];
        assert_eq!(create_user.body_param.as_deref(), Some("CreateUserRequest"));
        assert!(create_user.return_type.error_types.contains("BadRequest"));
        assert!(
            create_user
                .return_type
                .error_variants
                .contains("BadRequest::EmailAlreadyExists")
        );

        let with_message = &info["with_message"];
        assert_eq!(with_message.body_param.as_deref(), Some("Payload"));
        assert_eq!(
            with_message.return_type.found_type.as_deref(),
            Some("HealthResponse")
        );

        let plain = &info["plain"];
        assert_eq!(plain.body_param.as_deref(), Some("Plain"));
        assert!(plain.return_type.found_type.is_none());

        assert!(info["unknown_return"].return_type.found_type.is_none());
        assert!(info["no_extractors"].return_type.found_type.is_none());
        assert!(!info["no_extractors"].requires_auth);
    }

    #[test]
    fn error_collection_covers_variants_and_skips_variables() {
        let file = parse_file(
            r#"
            fn mixed_errors() -> Response {
                if bad {
                    return Err(Unauthorized::MissingToken.into());
                }
                match choice {
                    _ => return Err(Forbidden.into()),
                }
            }
            fn struct_error() -> Response {
                Err(BadRequest { code: "x" }.into())
            }
            fn variable_return() -> Response {
                return err_value;
            }
            fn no_errors() -> Response {
                format::json(HealthResponse(false))
            }
            "#,
        );
        let info = extract_handler_info(&file);

        let mixed = &info["mixed_errors"];
        assert!(mixed.return_type.error_types.contains("Unauthorized"));
        assert!(
            mixed
                .return_type
                .error_variants
                .contains("Unauthorized::MissingToken")
        );
        assert!(mixed.return_type.error_variants.contains("Forbidden"));
        assert!(mixed.return_type.error_types.contains("Forbidden"));

        let struct_error = &info["struct_error"];
        assert!(
            struct_error
                .return_type
                .error_variants
                .contains("BadRequest")
        );
        assert!(struct_error.return_type.error_types.contains("BadRequest"));

        let variable = &info["variable_return"];
        assert!(variable.return_type.error_types.is_empty());
        assert!(variable.return_type.error_variants.is_empty());

        assert!(info["no_errors"].return_type.error_types.is_empty());
    }

    #[test]
    fn vec_of_builtin_type_is_not_importable() {
        let file = parse_file(
            r"
            fn counts() -> Response {
                format::json(Vec::<String>::new())
            }
            ",
        );
        let info = extract_handler_info(&file);
        let counts = &info["counts"];
        assert_eq!(
            counts.return_type.found_type.as_deref(),
            Some("Array<String>")
        );
        assert!(!counts.return_type.is_importable);
    }

    #[test]
    fn is_type_like_heuristic() {
        assert!(is_type_like("BadRequest::Missing"));
        assert!(is_type_like("NotFound"));
        assert!(!is_type_like("err_value"));
        assert!(!is_type_like(""));
    }

    #[test]
    fn visitor_stops_after_first_conversion() {
        let mut visitor = ReturnTypeVisitor { found: None };
        let expr: syn::Expr = syn::parse_quote! {
            {
                let _a = json(UserResponse::from(x));
                let _b = json(OtherResponse::from(y));
            }
        };
        visitor.visit_expr(&expr);
        assert_eq!(visitor.found, Some(("UserResponse".to_string(), true)));
    }

    #[test]
    fn non_call_conversions_are_ignored() {
        let mut visitor = ReturnTypeVisitor { found: None };
        let method_into: syn::Expr = syn::parse_quote!(value.into());
        visitor.analyze_conversion(&method_into);
        assert!(visitor.found.is_none());

        let plain_path: syn::Expr = syn::parse_quote!(value);
        visitor.analyze_conversion(&plain_path);
        assert!(visitor.found.is_none());

        let closure_call: syn::Expr = syn::parse_quote!((closure)(x));
        visitor.visit_expr(&closure_call);
        assert!(visitor.found.is_none());
    }

    #[test]
    fn handler_signature_edges_are_tolerated() {
        let file = parse_file(
            r"
            fn with_self(self) -> i32 { 3 }
            fn tuple_input(pair: (u8, u8)) -> i32 { 3 }
            fn tuple_body(body: Json<(u8, u8)>) -> i32 { 3 }
            fn plain_response() -> i32 { 3 }
            ",
        );
        let info = extract_handler_info(&file);
        assert!(info["with_self"].body_param.is_none());
        assert!(info["tuple_input"].body_param.is_none());
        assert!(info["tuple_body"].body_param.is_none());
        assert_eq!(info["plain_response"].return_type.found_type, None);
    }

    #[test]
    fn hand_built_paths_without_segments_are_tolerated() {
        let mut file: syn::File = syn::parse_quote! {
            fn demo(x: Json<String>) -> i32 { 3 }
        };
        if let Some(syn::Item::Fn(func)) = file.items.first_mut()
            && let Some(syn::FnArg::Typed(pat_type)) = func.sig.inputs.first_mut()
        {
            *pat_type.ty = syn::Type::Path(syn::TypePath {
                attrs: Vec::new(),
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments: syn::punctuated::Punctuated::new(),
                },
            });
        }
        let info = extract_handler_info(&file);
        assert!(info["demo"].body_param.is_none());
        assert!(info["demo"].query_params.is_none());
        assert!(!info["demo"].requires_auth);
    }

    #[test]
    fn analyze_conversion_guards_cover_all_early_returns() {
        let mut visitor = ReturnTypeVisitor { found: None };
        let first: syn::Expr = syn::parse_quote!(UserResponse::from(user));
        visitor.analyze_conversion(&first);
        assert!(visitor.found.is_some());
        // Guard: a second conversion is ignored once a type was found.
        let second: syn::Expr = syn::parse_quote!(Other::from(x));
        visitor.analyze_conversion(&second);
        assert_eq!(visitor.found, Some(("UserResponse".to_string(), true)));

        // Guard: a fresh visitor ignores calls whose callee is not a path.
        let mut fresh = ReturnTypeVisitor { found: None };
        let paren_call: syn::Expr = syn::parse_quote!((closure)(x));
        fresh.analyze_conversion(&paren_call);
        assert!(fresh.found.is_none());
    }

    #[test]
    fn error_collector_ignores_non_error_expressions() {
        let mut collector = super::ErrorCollector {
            error_types: std::collections::BTreeSet::new(),
            error_variants: std::collections::BTreeSet::new(),
        };
        let literal: syn::Expr = syn::parse_quote!(42);
        collector.collect(&literal);
        assert!(collector.error_types.is_empty());
        assert!(collector.error_variants.is_empty());
    }

    #[test]
    fn vec_conversion_requires_generic_argument() {
        let plain: syn::Path = syn::parse_quote!(Vec::new);
        assert!(vec_conversion(&plain).is_none());
        let non_vec: syn::Path = syn::parse_quote!(Option::<String>::None);
        assert!(vec_conversion(&non_vec).is_none());
        let with_generic: syn::Path = syn::parse_quote!(Vec::<String>);
        let converted = vec_conversion(&with_generic);
        assert_eq!(converted, Some(("Array<String>".to_string(), false)));
        let non_path_generic: syn::Path = syn::parse_quote!(Vec::<()>);
        assert!(vec_conversion(&non_path_generic).is_none());
    }
}
