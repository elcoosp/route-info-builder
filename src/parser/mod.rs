mod handlers;

use crate::config::Config;
use crate::parser::handlers::ReturnTypeVisitor;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouteInfo {
    pub name: String,
    pub path: String,
    pub method: String,
    pub handler: String,
    pub handler_info: HandlerInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HandlerInfo {
    pub body_param: Option<String>,
    pub requires_auth: bool,
    pub return_type: handlers::ReturnTypeVisitor,
}

pub fn scan_controllers_folder(
    config: &Config,
) -> Result<Vec<RouteInfo>, Box<dyn std::error::Error>> {
    let controllers_dir = &config.controllers_path;
    let mut routes = Vec::new();

    let entries = fs::read_dir(controllers_dir).map_err(|e| {
        format!(
            "Failed to read controllers directory {}: {}",
            controllers_dir.display(),
            e
        )
    })?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && let Some(filename) = path.file_name().and_then(|s| s.to_str())
                && filename != "mod.rs" && filename.ends_with(".rs")
                    && let Some(file_routes) = parse_routes_from_file(&path, config)? {
                        routes.extend(file_routes);
                    }
    }

    // Deduplicate routes by (method, path) combination
    let mut seen = HashSet::new();
    routes.retain(|route| {
        let key = (route.method.clone(), route.path.clone());
        if seen.contains(&key) {
            eprintln!(
                "cargo:warning=Duplicate route skipped: {} {}",
                route.method, route.path
            );
            false
        } else {
            seen.insert(key);
            true
        }
    });

    Ok(routes)
}

fn parse_routes_from_file(
    file_path: &Path,
    config: &Config,
) -> Result<Option<Vec<RouteInfo>>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let syntax = syn::parse_file(&content)
        .map_err(|e| format!("Failed to parse {}: {}", file_path.display(), e))?;

    let mut routes = Vec::new();

    for item in &syntax.items {
        if let syn::Item::Fn(func) = item
            && func.sig.ident == "routes"
                && let Some(routes_vec) = extract_routes_from_axum_function(func, config)? {
                    routes = routes_vec;
                    break;
                }
    }

    // Now extract body parameters and auth requirements from handler functions
    let handler_info_map = handlers::extract_handler_info(&syntax)?;

    // Update routes with handler information
    for route in &mut routes {
        if let Some(info) = handler_info_map.get(&route.handler) {
            route.handler_info = info.clone(); // Set the complete HandlerInfo
        }
    }

    if routes.is_empty() {
        Ok(None)
    } else {
        Ok(Some(routes))
    }
}

fn extract_routes_from_axum_function(
    func: &syn::ItemFn,
    config: &Config,
) -> Result<Option<Vec<RouteInfo>>, Box<dyn std::error::Error>> {
    let block = &func.block;
    let mut routes = Vec::new();
    let mut current_prefix = String::new();

    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                extract_routes_from_expr(expr, &mut routes, &mut current_prefix, config)?;
            }
            _ => {} // Skip other statement types
        }
    }

    if routes.is_empty() {
        Ok(None)
    } else {
        Ok(Some(routes))
    }
}

fn extract_routes_from_expr(
    expr: &syn::Expr,
    routes: &mut Vec<RouteInfo>,
    prefix: &mut String,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    match expr {
        syn::Expr::MethodCall(method_call) => {
            let method_name = method_call.method.to_string();

            // FIRST process the receiver to establish context (including any prefixes)
            extract_routes_from_expr(&method_call.receiver, routes, prefix, config)?;

            // THEN process the current method call
            match method_name.as_str() {
                "prefix" => {
                    if let Some(first_arg) = method_call.args.first()
                        && let syn::Expr::Lit(expr_lit) = first_arg
                            && let syn::Lit::Str(lit_str) = &expr_lit.lit {
                                *prefix = lit_str.value();
                                // Ensure prefix starts with slash
                                if !prefix.starts_with('/') {
                                    *prefix = format!("/{}", prefix);
                                }
                            }
                }
                "add" => {
                    if let (Some(path_expr), Some(method_expr)) =
                        (method_call.args.get(0), method_call.args.get(1))
                    {
                        let path = extract_string_literal(path_expr)
                            .ok_or("Failed to extract path from add() call")?;
                        let (method, handler) = extract_http_method_and_handler(method_expr)
                            .ok_or("Failed to extract HTTP method and handler from add() call")?;

                        // Build full path with current prefix
                        let full_path = crate::utils::path::build_full_path(prefix, &path);

                        let name = crate::config::naming::generate_route_name(
                            &full_path,
                            &method,
                            &config.naming,
                        );

                        routes.push(RouteInfo {
                            name,
                            path: full_path,
                            method,
                            handler,
                            handler_info: HandlerInfo {
                                body_param: None,
                                requires_auth: false,
                                return_type: ReturnTypeVisitor::default(),
                            },
                        });
                    }
                }
                _ => {}
            }
        }
        syn::Expr::Call(call_expr) => {
            // Handle Routes::new() call - reset prefix
            if let syn::Expr::Path(func_path) = &*call_expr.func
                && let Some(segment) = func_path.path.segments.last()
                    && segment.ident == "new" {
                        *prefix = String::new(); // Reset prefix for new chain
                    }
        }
        _ => {}
    }

    Ok(())
}

fn extract_string_literal(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(expr_lit) = expr
        && let syn::Lit::Str(lit_str) = &expr_lit.lit {
            return Some(lit_str.value());
        }
    None
}

/// Extract both HTTP method and handler function name
fn extract_http_method_and_handler(expr: &syn::Expr) -> Option<(String, String)> {
    if let syn::Expr::Call(call_expr) = expr
        && let syn::Expr::Path(func_path) = &*call_expr.func
            && let Some(segment) = func_path.path.segments.last() {
                let method_name = segment.ident.to_string().to_uppercase();

                // Extract handler function name from arguments
                if let Some(handler_expr) = call_expr.args.first()
                    && let syn::Expr::Path(handler_path) = handler_expr
                        && let Some(handler_segment) = handler_path.path.segments.last() {
                            let handler_name = handler_segment.ident.to_string();
                            return Some((method_name, handler_name));
                        }
            }
    None
}
