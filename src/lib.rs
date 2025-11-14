use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    /// Path to the controllers directory to scan
    pub controllers_path: PathBuf,
    /// Optional output file name (default: "links.rs")
    pub output_file: Option<String>,
    /// Whether to include HTTP methods in variant names
    pub include_method_in_names: Option<bool>,
    /// Custom prefix to remove from paths when generating names
    pub path_prefix_to_remove: Option<String>,
    /// Case for variant names (default: "PascalCase")
    pub variant_case: Option<String>,
    /// Case for field names (default: "snake_case")
    pub field_case: Option<String>,
    /// Characters to treat as word separators in route names
    pub word_separators: Option<String>,
    /// Whether to preserve numbers as separate words
    pub preserve_numbers: Option<bool>,
    /// Custom prefix for variant names
    pub variant_prefix: Option<String>,
    /// Custom suffix for variant names
    pub variant_suffix: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub name: String,
    pub path: String,
    pub method: String,
}

/// Main function to generate links enum from controller files
pub fn generate_links(config: &Config) -> Result<String, Box<dyn std::error::Error>> {
    let routes = scan_controllers_folder(config)?;
    let generated_code = generate_links_enum(&routes, config);
    Ok(generated_code)
}

fn scan_controllers_folder(config: &Config) -> Result<Vec<RouteInfo>, Box<dyn std::error::Error>> {
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

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                if filename != "mod.rs" && filename.ends_with(".rs") {
                    if let Some(file_routes) = parse_routes_from_file(&path, config)? {
                        routes.extend(file_routes);
                    }
                }
            }
        }
    }

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

    for item in syntax.items {
        if let syn::Item::Fn(func) = item {
            if func.sig.ident == "routes" {
                if let Some(routes_vec) = extract_routes_from_axum_function(&func, config)? {
                    routes = routes_vec;
                    break;
                }
            }
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

            match method_name.as_str() {
                "prefix" => {
                    if let Some(first_arg) = method_call.args.first() {
                        if let syn::Expr::Lit(expr_lit) = first_arg {
                            if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                                *prefix = lit_str.value();
                            }
                        }
                    }
                    extract_routes_from_expr(&method_call.receiver, routes, prefix, config)?;
                }
                "add" => {
                    if let (Some(path_expr), Some(method_expr)) =
                        (method_call.args.get(0), method_call.args.get(1))
                    {
                        let path = extract_string_literal(path_expr)
                            .ok_or("Failed to extract path from add() call")?;
                        let method = extract_http_method(method_expr)
                            .ok_or("Failed to extract HTTP method from add() call")?;

                        let full_path = if prefix.is_empty() {
                            path.clone()
                        } else {
                            format!("{}{}", prefix, path)
                        };

                        let name = generate_route_name(&full_path, &method, config);

                        routes.push(RouteInfo {
                            name,
                            path: full_path,
                            method,
                        });
                    }
                    extract_routes_from_expr(&method_call.receiver, routes, prefix, config)?;
                }
                "new" => {
                    // Routes::new() - nothing to extract
                }
                _ => {
                    extract_routes_from_expr(&method_call.receiver, routes, prefix, config)?;
                }
            }
        }
        syn::Expr::Call(call_expr) => {
            // Handle Routes::new() call
            if let syn::Expr::Path(path_expr) = &*call_expr.func {
                if let Some(segment) = path_expr.path.segments.last() {
                    if segment.ident == "new" {
                        // This is the start of the chain
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn extract_string_literal(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(expr_lit) = expr {
        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
            return Some(lit_str.value());
        }
    }
    None
}

fn extract_http_method(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Call(call_expr) = expr {
        if let syn::Expr::Path(path_expr) = &*call_expr.func {
            if let Some(segment) = path_expr.path.segments.last() {
                let method_name = segment.ident.to_string();
                return Some(method_name.to_uppercase());
            }
        }
    }
    None
}

fn generate_route_name(path: &str, method: &str, config: &Config) -> String {
    let include_method = config.include_method_in_names.unwrap_or(true);

    // Remove prefix if specified
    let mut processed_path = path.to_string();
    if let Some(prefix) = &config.path_prefix_to_remove {
        if processed_path.starts_with(prefix) {
            processed_path = processed_path[prefix.len()..].to_string();
        }
    }

    // Clean the path for name generation
    let clean_path = clean_route_path_for_name(&processed_path, config);

    let base_name = if clean_path.is_empty() {
        "root".to_string()
    } else {
        clean_path
    };

    let name = if include_method {
        format!("{}_{}", method.to_lowercase(), base_name)
    } else {
        base_name
    };

    // Apply final transformations to ensure valid identifier
    sanitize_identifier(&name)
}

fn clean_route_path_for_name(path: &str, config: &Config) -> String {
    let mut result = path.trim_matches('/').replace("//", "/");

    // Replace parameter placeholders
    result = result.replace('{', "").replace('}', "");

    // Use custom word separators if specified
    if let Some(separators) = &config.word_separators {
        for sep in separators.chars() {
            result = result.replace(sep, "_");
        }
    } else {
        // Default separators
        result = result.replace(['-', '/', '.', ':'], "_");
    }

    // Remove duplicate underscores and trim
    while result.contains("__") {
        result = result.replace("__", "_");
    }

    result.trim_matches('_').to_string()
}

fn sanitize_identifier(name: &str) -> String {
    let mut result = String::new();
    let mut chars = name.chars().peekable();

    // Ensure the identifier starts with a letter or underscore
    if let Some(&first) = chars.peek() {
        if !first.is_alphabetic() && first != '_' {
            result.push('_');
        }
    }

    for c in chars {
        if c.is_alphanumeric() || c == '_' {
            result.push(c);
        } else {
            result.push('_');
        }
    }

    result
}

fn generate_links_enum(routes: &[RouteInfo], config: &Config) -> String {
    let mut variants = Vec::new();
    let mut match_arms = Vec::new();
    let mut method_arms = Vec::new();

    for route in routes {
        let variant_name = create_variant_name(&route.name, config);
        let route_path = route.path.clone();
        let route_method = route.method.clone();

        // Extract parameters from path (e.g., {id})
        let path_params = extract_parameters_from_path(&route.path);

        if path_params.is_empty() {
            // No parameters variant
            variants.push(quote! {
                #variant_name
            });

            match_arms.push(quote! {
                Link::#variant_name => #route_path.to_string()
            });
        } else {
            // With parameters variant
            let fields: Vec<proc_macro2::Ident> = path_params
                .iter()
                .map(|param| {
                    let field_name = create_field_name(param, config);
                    syn::Ident::new(&field_name, proc_macro2::Span::call_site())
                })
                .collect();

            let field_declarations: Vec<_> = fields
                .iter()
                .map(|field| quote! { #field: String })
                .collect();

            let field_patterns: Vec<_> = fields.iter().map(|field| quote! { #field }).collect();

            // Build the path replacement logic
            let path_build_code = generate_path_build_code(&route_path, &fields);

            variants.push(quote! {
                #variant_name {
                    #(#field_declarations),*
                }
            });

            match_arms.push(quote! {
                Link::#variant_name { #(#field_patterns),* } => {
                    #path_build_code
                }
            });
        }

        method_arms.push(quote! {
            Link::#variant_name { .. } => #route_method
        });
    }

    let generated = quote! {
        /// Auto-generated link enum for all application routes
        #[derive(Debug, Clone, PartialEq)]
        pub enum Link {
            #(#variants),*
        }

        impl Link {
            /// Convert the link to a URL path string
            pub fn to_path(&self) -> String {
                match self {
                    #(#match_arms),*
                }
            }

            /// Get the HTTP method for this route
            pub fn method(&self) -> &'static str {
                match self {
                    #(#method_arms),*
                }
            }
        }

        impl std::fmt::Display for Link {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.to_path())
            }
        }
    };

    generated.to_string()
}

fn create_variant_name(name: &str, config: &Config) -> proc_macro2::Ident {
    let case = config.variant_case.as_deref().unwrap_or("pascal");
    let mut result = convert_case(name, case);

    // Apply prefix and suffix
    if let Some(prefix) = &config.variant_prefix {
        result = format!("{}{}", prefix, result);
    }

    if let Some(suffix) = &config.variant_suffix {
        result = format!("{}{}", result, suffix);
    }

    // Ensure it's a valid identifier
    let sanitized = sanitize_identifier(&result);
    syn::Ident::new(&sanitized, proc_macro2::Span::call_site())
}

fn create_field_name(name: &str, config: &Config) -> String {
    let case = config.field_case.as_deref().unwrap_or("snake");
    let result = convert_case(name, case);
    sanitize_identifier(&result)
}

fn convert_case(input: &str, case: &str) -> String {
    match case.to_lowercase().as_str() {
        "pascal" | "pascalcase" => input.to_case(Case::Pascal),
        "camel" | "camelcase" => input.to_case(Case::Camel),
        "snake" | "snake_case" => input.to_case(Case::Snake),
        "kebab" | "kebab-case" => input.to_case(Case::Kebab),
        "screaming_snake" | "screaming_snake_case" | "upper_snake" => {
            input.to_case(Case::ScreamingSnake)
        }
        "title" | "title_case" => input.to_case(Case::Title),
        "lower" | "lowercase" => input.to_lowercase(),
        "upper" | "uppercase" => input.to_uppercase(),
        _ => input.to_case(Case::Pascal), // default
    }
}

fn generate_path_build_code(path_template: &str, fields: &[proc_macro2::Ident]) -> TokenStream {
    let mut path_parts = Vec::new();

    for segment in path_template.split('/') {
        if segment.is_empty() {
            continue;
        }

        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = &segment[1..segment.len() - 1];
            let field_ident = syn::Ident::new(
                &param_name.to_case(Case::Snake),
                proc_macro2::Span::call_site(),
            );

            // Verify this field exists
            if fields.iter().any(|f| f == &field_ident) {
                path_parts.push(quote! { #field_ident.clone() });
            } else {
                path_parts.push(quote! { #segment.to_string() });
            }
        } else {
            path_parts.push(quote! { #segment.to_string() });
        }
    }

    if path_parts.is_empty() {
        quote! { "/".to_string() }
    } else {
        quote! {
            let mut path = String::new();
            #(
                if !path.is_empty() {
                    path.push('/');
                }
                path.push_str(&#path_parts);
            )*
            if path.is_empty() {
                path.push('/');
            }
            path
        }
    }
}

fn extract_parameters_from_path(path: &str) -> Vec<String> {
    let mut params = Vec::new();

    for segment in path.split('/') {
        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = segment[1..segment.len() - 1].to_string();
            params.push(param_name);
        }
    }

    // Remove duplicates while preserving order
    let mut seen = HashSet::new();
    params.retain(|param| seen.insert(param.clone()));

    params
}
