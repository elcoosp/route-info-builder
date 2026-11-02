//! Extracts routes from the body of a controller `routes()`-style function.
//!
//! Both registration styles are supported:
//!
//! * Loco style: `Routes::new().prefix("/api").add("/users", get(list_users))`
//! * Axum style: `Router::new().route("/users", get(list_users))`
//!   plus `.nest(...)` and `.merge(...)`.
//!
//! Method-router chains such as `get(one).post(two)` emit one route per verb.

use crate::configuration::Config;
use crate::configuration::naming;
use crate::error::Result;
use crate::parser::{HandlerInfo, RouteInfo};
use crate::utils::path;
use crate::utils::path::build_full_path;

/// HTTP verbs recognized in method-router calls.
const HTTP_VERBS: &[&str] = &[
    "get", "post", "put", "patch", "delete", "head", "options", "trace", "connect",
];

/// Extraction context carrying the routes collected so far and the prefix
/// stack maintained while walking `nest` calls.
struct Extractor<'config> {
    routes: Vec<RouteInfo>,
    prefix: String,
    nest_base: Vec<String>,
    config: &'config Config,
}

/// Extracts all routes declared in `routes_function`.
pub(crate) fn extract(routes_function: &syn::ItemFn, config: &Config) -> Result<Vec<RouteInfo>> {
    let mut extractor = Extractor {
        routes: Vec::new(),
        prefix: String::new(),
        nest_base: Vec::new(),
        config,
    };
    for statement in &routes_function.block.stmts {
        if let syn::Stmt::Expr(expr, _) = statement {
            extractor.visit_expr(expr)?;
        }
    }
    Ok(extractor.routes)
}

impl Extractor<'_> {
    /// Walks an expression looking for router-building method calls.
    fn visit_expr(&mut self, expr: &syn::Expr) -> Result<()> {
        match expr {
            syn::Expr::MethodCall(call) => self.visit_method_call(call),
            syn::Expr::Call(call) => self.visit_call(call),
            _ => Ok(()),
        }
    }

    /// Handles `Routes::new()` (prefix reset) and recurses into call args.
    fn visit_call(&mut self, call: &syn::ExprCall) -> Result<()> {
        if let syn::Expr::Path(func_path) = &*call.func
            && let Some(segment) = func_path.path.segments.last()
            && segment.ident == "new"
        {
            self.prefix = self.nest_base.last().cloned().unwrap_or_default();
        }
        for arg in &call.args {
            self.visit_expr(arg)?;
        }
        Ok(())
    }

    /// Handles `prefix`, `add`, `route`, `nest` and `merge` calls.
    fn visit_method_call(&mut self, call: &syn::ExprMethodCall) -> Result<()> {
        self.visit_expr(&call.receiver)?;
        let method_name = call.method.to_string();
        match method_name.as_str() {
            "prefix" => {
                if let Some(literal) = string_literal(call.args.first()) {
                    self.prefix = ensure_leading_slash(&literal);
                }
            }
            "add" | "route" => {
                if let (Some(path_literal), Some(method_router)) =
                    (string_literal(call.args.first()), call.args.get(1))
                {
                    let prefix = self.prefix.clone();
                    self.push_method_router(&prefix, &path_literal, method_router)?;
                }
            }
            "nest" => {
                if let (Some(path_literal), Some(inner)) =
                    (string_literal(call.args.first()), call.args.get(1))
                {
                    let previous = self.prefix.clone();
                    let nested = build_full_path(&previous, &ensure_leading_slash(&path_literal));
                    self.prefix.clone_from(&nested);
                    self.nest_base.push(nested);
                    self.visit_expr(inner)?;
                    self.nest_base.pop();
                    self.prefix = previous;
                }
            }
            "merge" => {
                if let Some(inner) = call.args.first() {
                    self.visit_expr(inner)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Unwraps `get(handler)` and `get(one).post(two)` chains in source order.
    fn push_method_router(
        &mut self,
        prefix: &str,
        route_path: &str,
        method_router: &syn::Expr,
    ) -> Result<()> {
        match method_router {
            syn::Expr::MethodCall(call) => {
                self.push_method_router(prefix, route_path, &call.receiver)?;
                let verb = call.method.to_string();
                if HTTP_VERBS.contains(&verb.as_str())
                    && let Some(handler) = handler_name(call.args.first())
                {
                    self.push_route(prefix, route_path, &verb, &handler);
                }
            }
            syn::Expr::Call(call) => {
                if let Some(verb) = callee_name(&call.func)
                    && HTTP_VERBS.contains(&verb.as_str())
                    && let Some(handler) = handler_name(call.args.first())
                {
                    self.push_route(prefix, route_path, &verb, &handler);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Registers a single route with its generated name.
    fn push_route(&mut self, prefix: &str, route_path: &str, verb: &str, handler: &str) {
        let full_path = path::normalize_path(&build_full_path(prefix, route_path));
        let method = verb.to_uppercase();
        let name = naming::generate_route_name(&full_path, &method, &self.config.naming);
        self.routes.push(RouteInfo {
            name,
            path: full_path,
            method,
            handler: handler.to_string(),
            handler_info: HandlerInfo::default(),
        });
    }
}

/// Ensures the string starts with a slash.
fn ensure_leading_slash(value: &str) -> String {
    if value.starts_with('/') {
        value.to_string()
    } else {
        format!("/{value}")
    }
}

/// Returns the string value of the first literal in `expr`.
fn string_literal(expr: Option<&syn::Expr>) -> Option<String> {
    match expr? {
        syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(lit_str) => Some(lit_str.value()),
            _ => None,
        },
        _ => None,
    }
}

/// Returns the last path segment of a callee, e.g. `get` in `get(handler)`.
fn callee_name(func: &syn::Expr) -> Option<String> {
    match func {
        syn::Expr::Path(func_path) => func_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        _ => None,
    }
}

/// Returns the handler function name from `get(handler)` arguments.
fn handler_name(expr: Option<&syn::Expr>) -> Option<String> {
    match expr? {
        syn::Expr::Path(handler_path) => handler_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_leading_slash, extract, handler_name, string_literal};
    use crate::configuration::Config;
    use crate::error::Result;
    use quote::quote;

    fn parse_fn(tokens: proc_macro2::TokenStream) -> syn::ItemFn {
        syn::parse2(tokens).expect("fixture must parse")
    }

    fn extract_routes(tokens: proc_macro2::TokenStream) -> Result<Vec<crate::parser::RouteInfo>> {
        extract(&parse_fn(tokens), &Config::default())
    }

    #[test]
    fn loco_style_chain_is_extracted() -> Result<()> {
        let tokens = quote! {
            fn routes() -> Routes {
                Routes::new()
                    .prefix("/api")
                    .add("/users", get(list_users))
                    .add("/users/{user_id}", get(get_user))
                    .add("/users", post(create_user))
            }
        };
        let routes = extract_routes(tokens)?;
        assert_eq!(routes.len(), 3);
        assert_eq!(routes[0].path, "/api/users");
        assert_eq!(routes[0].method, "GET");
        assert_eq!(routes[0].handler, "list_users");
        assert_eq!(routes[1].path, "/api/users/{user_id}");
        assert_eq!(routes[2].method, "POST");
        Ok(())
    }

    #[test]
    fn axum_style_chain_with_nest_and_merge_is_extracted() -> Result<()> {
        let tokens = quote! {
            fn routes() -> Router {
                Router::new()
                    .route("/posts", get(list_posts))
                    .nest("/api", Router::new().route("/health", get(health)))
                    .merge(other_routes())
            }
        };
        let routes = extract_routes(tokens)?;
        let paths: Vec<&str> = routes.iter().map(|route| route.path.as_str()).collect();
        assert_eq!(paths, vec!["/posts", "/api/health"]);
        Ok(())
    }

    #[test]
    fn degenerate_router_calls_are_ignored() -> Result<()> {
        let tokens = quote! {
            fn routes() -> Router {
                let method_router = get(indirect);
                Router::new()
                    .nest("/lonely")
                    .merge()
                    .unknown_method("whatever")
                    .route("/paren", (get)(paren_handler))
                    .route("/var", method_router)
                    .route("/direct", get(direct_handler))
            }
        };
        let routes = extract_routes(tokens)?;
        // Only the direct registration is recognized; variables and
        // degenerate calls are skipped gracefully.
        let paths: Vec<&str> = routes.iter().map(|route| route.path.as_str()).collect();
        assert_eq!(paths, vec!["/direct"]);
        Ok(())
    }

    #[test]
    fn verb_chains_emit_one_route_per_verb() -> Result<()> {
        let tokens = quote! {
            fn routes() -> Router {
                Router::new().route("/things", get(list_things).post(create_thing))
            }
        };
        let routes = extract_routes(tokens)?;
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].method, "GET");
        assert_eq!(routes[0].handler, "list_things");
        assert_eq!(routes[1].method, "POST");
        assert_eq!(routes[1].handler, "create_thing");
        Ok(())
    }

    #[test]
    fn legacy_param_syntax_is_normalized() -> Result<()> {
        let tokens = quote! {
            fn routes() -> Router {
                Router::new()
                    .route("/users/:user_id", get(get_user))
                    .route("/files/*rest", get(serve_file))
            }
        };
        let routes = extract_routes(tokens)?;
        assert_eq!(routes[0].path, "/users/{user_id}");
        assert_eq!(routes[1].path, "/files/{*rest}");
        Ok(())
    }

    #[test]
    fn non_string_paths_and_unknown_verbs_are_skipped() -> Result<()> {
        let tokens = quote! {
            fn routes() -> Router {
                let dynamic = format!("/{}", "x");
                Router::new()
                    .route(dynamic, get(weird))
                    .route("/literal", unknown_verb(handler))
                    .route("/ok", get(ok_handler))
            }
        };
        let routes = extract_routes(tokens)?;
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/ok");
        Ok(())
    }

    #[test]
    fn statements_and_new_calls_reset_prefix() -> Result<()> {
        let tokens = quote! {
            fn routes() -> Routes {
                Routes::new().prefix("/api").add("/users", get(list_users));
                Routes::new().add("/other", get(other_handler))
            }
        };
        let routes = extract_routes(tokens)?;
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].path, "/api/users");
        assert_eq!(routes[1].path, "/other");
        Ok(())
    }

    #[test]
    fn body_without_router_calls_yields_nothing() -> Result<()> {
        let tokens = quote! {
            fn routes() -> Router {
                let answer = 42;
                some_unrelated_call(answer)
            }
        };
        let routes = extract_routes(tokens)?;
        assert_eq!(routes, Vec::new());
        Ok(())
    }

    #[test]
    fn helpers_cover_edge_branches() {
        assert_eq!(ensure_leading_slash("/x"), "/x");
        assert_eq!(ensure_leading_slash("x"), "/x");
        let literal: Option<syn::Expr> = Some(syn::parse_quote!("/lit"));
        assert_eq!(string_literal(literal.as_ref()), Some("/lit".to_string()));
        let number: Option<syn::Expr> = Some(syn::parse_quote!(42));
        assert_eq!(string_literal(number.as_ref()), None);
        assert_eq!(string_literal(None), None);
        let handler: Option<syn::Expr> = Some(syn::parse_quote!(handlers::list_users));
        assert_eq!(
            handler_name(handler.as_ref()),
            Some("list_users".to_string())
        );
        let call: Option<syn::Expr> = Some(syn::parse_quote!(build_handler(1)));
        assert_eq!(handler_name(call.as_ref()), None);
    }
}
