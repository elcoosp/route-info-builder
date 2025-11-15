use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use ts_quote::ts_string;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    /// Path to the controllers directory to scan
    pub controllers_path: PathBuf,
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
    /// Optional output path for TypeScript client
    pub typescript_client_output: Option<PathBuf>,
    /// Whether to generate TypeScript client
    pub generate_typescript_client: Option<bool>,
}

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
    pub return_type: Option<String>,
}

/// Main function to generate links enum from controller files
pub fn generate_links(config: &Config) -> Result<String, Box<dyn std::error::Error>> {
    let routes = scan_controllers_folder(config)?;

    // Generate Rust links enum
    let rust_code = generate_links_enum(&routes, config);

    // Generate TypeScript client if requested
    if config.generate_typescript_client.unwrap_or(false) {
        if let Some(ts_output) = &config.typescript_client_output {
            let ts_code = generate_ts_client_code(&routes)?;
            fs::write(ts_output, ts_code)?;
            println!(
                "cargo:warning=Generated TypeScript client at: {}",
                ts_output.display()
            );
        }
    }

    Ok(rust_code)
}

/// Generate TypeScript HTTP client compatible with tanstack-query
pub fn generate_ts_client(config: &Config) -> Result<String, Box<dyn std::error::Error>> {
    let routes = scan_controllers_folder(config)?;
    let ts_code = generate_ts_client_code(&routes)?;
    Ok(ts_code)
}

fn generate_ts_client_code(routes: &[RouteInfo]) -> Result<String, Box<dyn std::error::Error>> {
    let mut imports = Vec::new();
    let mut client_methods = Vec::new();
    let mut hooks = Vec::new();
    let mut interfaces = Vec::new();
    let mut type_imports = HashSet::new(); // Track types to import

    // Generate imports
    imports.push(ts_string! {
        import { useQuery, useMutation, type UseQueryOptions, type UseMutationOptions } from "@tanstack/react-query";
    });

    // Collect all unique body types for import
    for route in routes {
        if let Some(body_type) = &route.handler_info.body_param {
            type_imports.insert(body_type.clone());
        }
        if let Some(return_type) = &route.handler_info.return_type {
            type_imports.insert(return_type.clone());
        }
    }

    // Generate type imports
    for type_name in &type_imports {
        let import = format!("\"../../../bindings/{type_name}\"");
        imports.push(ts_string! {
            import { type #type_name } from #import;
        });
    }

    // Generate the base HTTP client with auth support
    let http_client = generate_http_client();

    for route in routes {
        let method_name = convert_case_ts(&route.name, "camel");
        let hook_name = format!("use{}", convert_case_ts(&route.name, "pascal"));
        let params = extract_parameters_from_path(&route.path);

        // Generate client method
        let client_method = generate_ts_client_method(route, &method_name, &params);
        client_methods.push(client_method);

        // Generate parameter interface if needed
        if !params.is_empty() {
            let interface = generate_ts_interface(&method_name, &params);
            interfaces.push(interface);
        }

        // Generate hook
        let hook = generate_ts_hook(route, &method_name, &hook_name, &params);
        hooks.push(hook);
    }

    let imports_str = imports.join("\n");
    let client_methods_str = client_methods.join("\n");
    let interfaces_str = interfaces.join("\n");
    let hooks_str = hooks.join("\n");

    // Combine all parts
    let ts_code = ts_string! {
        #imports_str

        // HTTP client with auth support
        #http_client

        // Client
        export const client = {
            #client_methods_str
        };

        // Interfaces
        #interfaces_str

        // Hooks
        #hooks_str
    };

    // Format the TypeScript code
    let formatted = format_ts_code(&ts_code.to_string())?;
    Ok(formatted)
}

/// Generate a reusable HTTP client with auth support
fn generate_http_client() -> String {
    ts_string! {
        // Base HTTP client with authentication support
        class ApiClient {
            private baseUrl: string = "";
            private getToken?: () => Promise<string | null>;

            constructor(config?: { baseUrl?: string; getToken?: () => Promise<string | null> }) {
                this.baseUrl = config?.baseUrl || "";
                this.getToken = config?.getToken;
            }

            async request<T>(url: string, options: RequestInit & { requiresAuth?: boolean } = {}): Promise<T> {
                const headers = new Headers(options.headers as Record<string, string>);

                // Set Content-Type for requests with body
                if (options.body && !headers.has("Content-Type")) {
                    headers.set("Content-Type", "application/json");
                }

                // Add Authorization header if required and token is available
                if (options.requiresAuth && this.getToken) {
                    const token = await this.getToken();
                    if (token) {
                        headers.set("Authorization", "Bearer " + token);
                    }
                }

                const response = await fetch(this.baseUrl+url, {
                    ...options,
                    headers,
                });

                if (!response.ok) {
                    throw new Error("HTTP error! status: "+response.status);
                }

                // For 204 No Content responses, return null
                if (response.status === 204) {
                    return null as T;
                }

                return response.json() as Promise<T>;
            }

            async get<T>(url: string, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}) {
                return this.request<T>(url, {
                    method: "GET",
                    requiresAuth: options.requiresAuth,
                    signal: options.signal,
                });
            }

            async post<T>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}) {
                return this.request<T>(url, {
                    method: "POST",
                    body: data ? JSON.stringify(data) : undefined,
                    requiresAuth: options.requiresAuth,
                    signal: options.signal,
                });
            }

            async put<T>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}) {
                return this.request<T>(url, {
                    method: "PUT",
                    body: data ? JSON.stringify(data) : undefined,
                    requiresAuth: options.requiresAuth,
                    signal: options.signal,
                });
            }

            async patch<T>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}) {
                return this.request<T>(url, {
                    method: "PATCH",
                    body: data ? JSON.stringify(data) : undefined,
                    requiresAuth: options.requiresAuth,
                    signal: options.signal,
                });
            }

            async delete<T>(url: string, data?: any,options: { requiresAuth?: boolean; signal?: AbortSignal } = {}) {
                return this.request<T>(url, {
                    method: "DELETE",
                    body: data ? JSON.stringify(data) : undefined,
                    requiresAuth: options.requiresAuth,
                    signal: options.signal,
                });
            }
        }

        // Create default instance
        export const apiClient = new ApiClient();

        // You can configure the client elsewhere in your app:
        // apiClient = new ApiClient({
        //   baseUrl: 'http://localhost:3000',
        //   getToken: () => authCtx.getToken()
        // });
    }
}

fn generate_ts_client_method<'a>(
    route: &'a RouteInfo,
    method_name: &'a str,
    params: &'a [String],
) -> String {
    let _method_upper = route.method.to_uppercase();
    let path_template = generate_ts_path_template(&route.path, params);

    // Use the actual body type or void for no body
    let body_type = route.handler_info.body_param.as_deref().unwrap_or("void");
    // Use the actual return type or any as fallback
    let return_type = route.handler_info.return_type.as_deref().unwrap_or("any");
    let requires_auth = route.handler_info.requires_auth;

    if params.is_empty() {
        if route.method == "GET" {
            ts_string! {
                #method_name: async (config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.get<#return_type>(url, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else {
            let method_call = match route.method.as_str() {
                "POST" => "post",
                "PUT" => "put",
                "PATCH" => "patch",
                "DELETE" => "delete",
                _ => "post",
            };

            ts_string! {
                #method_name: async (body: #body_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.#method_call<#return_type>(url, body, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        }
    } else {
        let params_type = format!("{}Params", convert_case_ts(method_name, "pascal"));

        if route.method == "GET" {
            ts_string! {
                #method_name: async (params: #params_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.get<#return_type>(url, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else {
            let method_call = match route.method.as_str() {
                "POST" => "post",
                "PUT" => "put",
                "PATCH" => "patch",
                "DELETE" => "delete",
                _ => "post",
            };

            ts_string! {
                #method_name: async (params: #params_type, body: #body_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.#method_call<#return_type>(url, body, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        }
    }
}

fn generate_ts_path_template(path: &str, _params: &[String]) -> String {
    let mut template = String::new();
    let segments: Vec<&str> = path.split('/').collect();

    for (i, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            continue;
        }

        if i > 0 {
            template.push('/');
        }

        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = &segment[1..segment.len() - 1];
            let ts_param_name = convert_case_ts(param_name, "camel");
            template.push_str(&format!("${{params.{}}}", ts_param_name));
        } else {
            template.push_str(segment);
        }
    }

    let temp = if template.is_empty() {
        "/".to_string()
    } else if !template.starts_with('/') {
        format!("/{}", template)
    } else {
        template
    };
    format!("`{temp}`")
}

fn generate_ts_interface(method_name: &str, params: &[String]) -> String {
    let interface_name = format!("{}Params", convert_case_ts(method_name, "pascal"));
    let mut fields = Vec::new();

    for param in params {
        let field_name = convert_case_ts(param, "camel");
        fields.push(ts_string! {
            #field_name: string;
        });
    }
    let fields_str = fields.join("\n");
    ts_string! {
        interface #interface_name {
            #fields_str
        }
    }
}

fn generate_ts_hook(
    route: &RouteInfo,
    method_name: &str,
    hook_name: &str,
    params: &[String],
) -> String {
    let method_name_str = format!("\"{method_name}\"");
    let body_type = route.handler_info.body_param.as_deref().unwrap_or("void");
    let return_type = route.handler_info.return_type.as_deref().unwrap_or("any");
    let _requires_auth = route.handler_info.requires_auth;

    if route.method == "GET" {
        if params.is_empty() {
            ts_string! {
                export function #hook_name(options?: UseQueryOptions<#return_type, Error>) {
                    return useQuery({
                        queryKey: [#method_name_str],
                        queryFn: ({ signal }) => client.#method_name({ signal }),
                        ...options,
                    });
                }
            }
            .into()
        } else {
            let params_type = format!("{}Params", convert_case_ts(method_name, "pascal"));
            ts_string! {
                export function #hook_name(params: #params_type, options?: UseQueryOptions<#return_type, Error>) {
                    return useQuery({
                        queryKey: [#method_name_str, params],
                        queryFn: ({ signal }) => client.#method_name(params, { signal }),
                        ...options,
                    });
                }
            }
        }
    } else {
        // Mutation hook - use proper body and return types
        if params.is_empty() {
            ts_string! {
                export function #hook_name(options?: UseMutationOptions<#return_type, Error, #body_type, unknown>) {
                    return useMutation({
                        mutationFn: (body: #body_type) => client.#method_name(body),
                        ...options,
                    });
                }
            }
            .into()
        } else {
            let params_type = format!("{}Params", convert_case_ts(method_name, "pascal"));
            ts_string! {
                export function #hook_name(options?: UseMutationOptions<#return_type, Error, { params: #params_type, body: #body_type }, unknown>) {
                    return useMutation({
                        mutationFn: (input: { params: #params_type, body: #body_type }) =>
                            client.#method_name(input.params, input.body),
                        ...options,
                    });
                }
            }
        }
    }
}

fn format_ts_code(code: &str) -> Result<String, Box<dyn std::error::Error>> {
    // For now, we'll use a simple formatter since deno_ast might be heavy
    Ok(code.to_string())
}

fn convert_case_ts(input: &str, case: &str) -> String {
    match case.to_lowercase().as_str() {
        "camel" | "camelcase" => input.to_case(Case::Camel),
        "pascal" | "pascalcase" => input.to_case(Case::Pascal),
        "snake" | "snake_case" => input.to_case(Case::Snake),
        "kebab" | "kebab-case" => input.to_case(Case::Kebab),
        _ => input.to_case(Case::Camel), // default to camelCase for TypeScript
    }
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
        if let syn::Item::Fn(func) = item {
            if func.sig.ident == "routes" {
                if let Some(routes_vec) = extract_routes_from_axum_function(func, config)? {
                    routes = routes_vec;
                    break;
                }
            }
        }
    }

    // Now extract body parameters and auth requirements from handler functions
    let handler_info_map = extract_handler_info(&syntax)?;

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
/// Extract body parameter types and auth requirements from handler functions
fn extract_handler_info(
    syntax: &syn::File,
) -> Result<HashMap<String, HandlerInfo>, Box<dyn std::error::Error>> {
    let mut handler_info = HashMap::new();

    for item in &syntax.items {
        if let syn::Item::Fn(func) = item {
            let handler_name = func.sig.ident.to_string();
            let mut body_param = None;
            let mut requires_auth = false;
            let mut return_type = None;

            // Look for Json, JsonValidate, and JsonValidateWithMessage parameters
            for input in &func.sig.inputs {
                if let syn::FnArg::Typed(pat_type) = input {
                    // Check for body parameters (Json<T>, JsonValidate<T>, JsonValidateWithMessage<T>)
                    if let syn::Type::Path(type_path) = &*pat_type.ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            let type_ident = segment.ident.to_string();

                            // Handle Json<T>, JsonValidate<T>, and JsonValidateWithMessage<T>
                            if matches!(
                                type_ident.as_str(),
                                "Json" | "JsonValidate" | "JsonValidateWithMessage"
                            ) {
                                // Extract the generic type parameter
                                if let syn::PathArguments::AngleBracketed(generics) =
                                    &segment.arguments
                                {
                                    if let Some(syn::GenericArgument::Type(syn::Type::Path(
                                        param_type,
                                    ))) = generics.args.first()
                                    {
                                        if let Some(param_segment) = param_type.path.segments.last()
                                        {
                                            body_param = Some(param_segment.ident.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Check for authentication (auth: JWT)
                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        if pat_ident.ident == "auth" {
                            if let syn::Type::Path(type_path) = &*pat_type.ty {
                                if let Some(segment) = type_path.path.segments.last() {
                                    if segment.ident == "JWT" {
                                        requires_auth = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // First try to extract return type from function body analysis
            return_type = extract_return_type_from_body(func);

            // If body analysis didn't work, fall back to signature analysis
            if return_type.is_none() {
                if let syn::ReturnType::Type(_, return_ty) = &func.sig.output {
                    return_type = extract_return_type(return_ty);
                }
            }

            handler_info.insert(
                handler_name,
                HandlerInfo {
                    body_param,
                    requires_auth,
                    return_type,
                },
            );
        }
    }

    Ok(handler_info)
}
fn extract_return_type(return_ty: &syn::Type) -> Option<String> {
    // Check guarded patterns first
    if let syn::Type::Path(type_path) = return_ty {
        if is_result_type(type_path) {
            return extract_type_from_generic(type_path, 0); // First type parameter is Ok type
        }
        if is_json_type(type_path) {
            return extract_type_from_generic(type_path, 0); // The type inside Json
        }
    }

    match return_ty {
        // Direct type like -> SwitchResponse
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                Some(segment.ident.to_string())
            } else {
                None
            }
        }
        // impl IntoResponse or other complex types - we can't easily determine
        syn::Type::ImplTrait(_) => None,
        _ => None,
    }
}
/// Extract return type by analyzing the function body to find format::json calls
fn extract_return_type_from_body(func: &syn::ItemFn) -> Option<String> {
    let mut visitor = ReturnTypeVisitor::default();
    visitor.visit_item_fn(func);
    visitor.found_type
}

#[derive(Default)]
struct ReturnTypeVisitor {
    found_type: Option<String>,
}

impl ReturnTypeVisitor {
    fn visit_expr(&mut self, expr: &syn::Expr) {
        if self.found_type.is_some() {
            return;
        }

        match expr {
            // format::json(Type::from(...))
            syn::Expr::Call(call_expr) => {
                if let syn::Expr::Path(path_expr) = &*call_expr.func {
                    if let Some(segment) = path_expr.path.segments.last() {
                        if segment.ident == "json" {
                            // This is format::json call
                            if let Some(first_arg) = call_expr.args.first() {
                                self.visit_conversion_expr(first_arg);
                                return; // Found our type, no need to continue
                            }
                        }
                    }
                }

                // Also visit all arguments recursively
                for arg in &call_expr.args {
                    self.visit_expr(arg);
                }
            }
            // Return statements
            syn::Expr::Return(return_expr) => {
                if let Some(expr) = &return_expr.expr {
                    self.visit_expr(expr);
                }
            }
            // Method calls
            syn::Expr::MethodCall(method_call) => {
                self.visit_expr(&method_call.receiver);
                for arg in &method_call.args {
                    self.visit_expr(arg);
                }
            }
            // Block expressions (like if blocks, match arms, etc.)
            syn::Expr::Block(block_expr) => {
                for stmt in &block_expr.block.stmts {
                    self.visit_stmt(stmt);
                }
                // Check for tail expression (last statement without semicolon)
                if let Some(syn::Stmt::Expr(expr, _)) = block_expr.block.stmts.last() {
                    self.visit_expr(expr);
                }
            }
            // If expressions
            syn::Expr::If(if_expr) => {
                self.visit_expr(&if_expr.cond);
                // Visit the then branch as a block
                for stmt in &if_expr.then_branch.stmts {
                    self.visit_stmt(stmt);
                }
                // Check for tail expression in then branch
                if let Some(syn::Stmt::Expr(expr, _)) = if_expr.then_branch.stmts.last() {
                    self.visit_expr(expr);
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.visit_expr(else_expr);
                }
            }
            // Match expressions
            syn::Expr::Match(match_expr) => {
                self.visit_expr(&match_expr.expr);
                for arm in &match_expr.arms {
                    self.visit_expr(&arm.body);
                }
            }
            _ => {}
        }
    }

    fn visit_conversion_expr(&mut self, expr: &syn::Expr) {
        match expr {
            // Type::from(value)
            syn::Expr::Call(call_expr) => {
                if let syn::Expr::Path(path_expr) = &*call_expr.func {
                    let path = &path_expr.path;
                    // Look for patterns like "SwitchResponse::from"
                    if path.segments.len() >= 2 {
                        if let Some(segment) = path.segments.iter().nth(path.segments.len() - 2) {
                            self.found_type = Some(segment.ident.to_string());
                        }
                    } else if let Some(segment) = path.segments.last() {
                        // Direct constructor call: Type(value)
                        self.found_type = Some(segment.ident.to_string());
                    }
                }
            }
            // Method calls on variables (like value.into())
            syn::Expr::MethodCall(method_call) => {
                if method_call.method == "into" {
                    // For into(), we can't easily determine the target type
                    // Skip and let signature analysis handle it
                }
            }
            // Simple path (like a variable name)
            syn::Expr::Path(_path_expr) => {
                // If we have a simple variable, we can't determine its type
                // without type inference, so we skip it
            }
            _ => {}
        }
    }

    fn visit_stmt(&mut self, stmt: &syn::Stmt) {
        if self.found_type.is_some() {
            return;
        }

        match stmt {
            syn::Stmt::Expr(expr, _) => {
                self.visit_expr(expr);
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.visit_expr(&init.expr);
                }
            }
            // Skip Item and Macro statements for now
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    fn visit_item_fn(&mut self, func: &syn::ItemFn) {
        for stmt in &func.block.stmts {
            self.visit_stmt(stmt);
        }
        // Check for tail expression in the function block
        if let Some(syn::Stmt::Expr(expr, _)) = func.block.stmts.last() {
            self.visit_expr(expr);
        }
    }
}
/// Extract the type from format::json(Type::from(...)) or similar patterns
fn extract_type_from_format_json_call(expr: &syn::Expr) -> Option<String> {
    match expr {
        // format::json(SwitchResponse::from(item))
        syn::Expr::Call(call_expr) => {
            if let syn::Expr::Path(path_expr) = &*call_expr.func {
                if is_format_json_path(&path_expr.path) {
                    if let Some(first_arg) = call_expr.args.first() {
                        return extract_type_from_conversion_call(first_arg);
                    }
                }
            }
        }
        // Return statement: return format::json(...)
        syn::Expr::Return(return_expr) => {
            if let Some(expr) = &return_expr.expr {
                return extract_type_from_format_json_call(expr);
            }
        }
        _ => {}
    }

    None
}

/// Extract type from conversion patterns like SwitchResponse::from(item)
fn extract_type_from_conversion_call(expr: &syn::Expr) -> Option<String> {
    match expr {
        // SwitchResponse::from(item)
        syn::Expr::Call(call_expr) => {
            if let syn::Expr::Path(path_expr) = &*call_expr.func {
                if let Some(segment) = path_expr.path.segments.last() {
                    if segment.ident == "from" {
                        // Get the type from the path before ::from
                        let type_path = &path_expr.path;
                        if type_path.segments.len() > 1 {
                            if let Some(type_segment) =
                                type_path.segments.iter().nth(type_path.segments.len() - 2)
                            {
                                return Some(type_segment.ident.to_string());
                            }
                        }
                    } else {
                        // Direct type constructor like SwitchResponse(item)
                        return Some(segment.ident.to_string());
                    }
                }
            }
        }
        // Variable that might be of the target type
        syn::Expr::Path(_) => {
            // This is trickier - we'd need type inference
            // For now, return None and rely on other methods
            return None;
        }
        _ => {}
    }

    None
}
/// Check if a type path is Result<T, E>
fn is_result_type(type_path: &syn::TypePath) -> bool {
    if let Some(segment) = type_path.path.segments.last() {
        segment.ident == "Result"
    } else {
        false
    }
}

/// Check if a type path is Json<T>
fn is_json_type(type_path: &syn::TypePath) -> bool {
    if let Some(segment) = type_path.path.segments.last() {
        segment.ident == "Json"
    } else {
        false
    }
}
fn is_format_json_path(path: &syn::Path) -> bool {
    if let Some(segment) = path.segments.last() {
        segment.ident == "json"
    } else {
        false
    }
}
/// Extract type from generic parameters at the given index
fn extract_type_from_generic(type_path: &syn::TypePath, index: usize) -> Option<String> {
    if let Some(segment) = type_path.path.segments.last() {
        if let syn::PathArguments::AngleBracketed(generics) = &segment.arguments {
            if let Some(syn::GenericArgument::Type(syn::Type::Path(param_type))) =
                generics.args.iter().nth(index)
            {
                if let Some(param_segment) = param_type.path.segments.last() {
                    return Some(param_segment.ident.to_string());
                }
            }
        }
    }
    None
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
    _config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    match expr {
        syn::Expr::MethodCall(method_call) => {
            let method_name = method_call.method.to_string();

            // FIRST process the receiver to establish context (including any prefixes)
            extract_routes_from_expr(&method_call.receiver, routes, prefix, _config)?;

            // THEN process the current method call
            match method_name.as_str() {
                "prefix" => {
                    if let Some(first_arg) = method_call.args.first() {
                        if let syn::Expr::Lit(expr_lit) = first_arg {
                            if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                                *prefix = lit_str.value();
                                // Ensure prefix starts with slash
                                if !prefix.starts_with('/') {
                                    *prefix = format!("/{}", prefix);
                                }
                            }
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
                        let full_path = build_full_path(prefix, &path);

                        let name = generate_route_name(&full_path, &method, _config);

                        routes.push(RouteInfo {
                            name,
                            path: full_path,
                            method,
                            handler,
                            handler_info: HandlerInfo {
                                body_param: None,
                                requires_auth: false,
                                return_type: None,
                            },
                        });
                    }
                }
                _ => {}
            }
        }
        syn::Expr::Call(call_expr) => {
            // Handle Routes::new() call - reset prefix
            if let syn::Expr::Path(func_path) = &*call_expr.func {
                if let Some(segment) = func_path.path.segments.last() {
                    if segment.ident == "new" {
                        *prefix = String::new(); // Reset prefix for new chain
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}
fn build_full_path(prefix: &str, path: &str) -> String {
    if prefix.is_empty() {
        path.to_string()
    } else {
        let clean_prefix = prefix.trim_end_matches('/');
        let clean_path = path.trim_start_matches('/');
        format!("{}/{}", clean_prefix, clean_path)
    }
}
fn extract_string_literal(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(expr_lit) = expr {
        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
            return Some(lit_str.value());
        }
    }
    None
}

/// Extract both HTTP method and handler function name
fn extract_http_method_and_handler(expr: &syn::Expr) -> Option<(String, String)> {
    if let syn::Expr::Call(call_expr) = expr {
        if let syn::Expr::Path(func_path) = &*call_expr.func {
            if let Some(segment) = func_path.path.segments.last() {
                let method_name = segment.ident.to_string().to_uppercase();

                // Extract handler function name from arguments
                if let Some(handler_expr) = call_expr.args.first() {
                    if let syn::Expr::Path(handler_path) = &*handler_expr {
                        if let Some(handler_segment) = handler_path.path.segments.last() {
                            let handler_name = handler_segment.ident.to_string();
                            return Some((method_name, handler_name));
                        }
                    }
                }
            }
        }
    }
    None
}

fn generate_route_name(path: &str, method: &str, config: &Config) -> String {
    let include_method = config.include_method_in_names.unwrap_or(true);

    // Create two versions of the path:
    // 1. The full path for the actual URL (includes prefix)
    // 2. The cleaned path for the name (prefix removed)

    let mut name_path = path.to_string();

    // Remove prefix from name but keep it in the actual route path
    if let Some(prefix) = &config.path_prefix_to_remove {
        let normalized_prefix = prefix.trim_matches('/');
        let normalized_path = name_path.trim_matches('/');

        if normalized_path.starts_with(normalized_prefix) {
            // Remove the prefix from the name but NOT from the actual path
            name_path = normalized_path[normalized_prefix.len()..].to_string();
            // Remove leading slash if present
            name_path = name_path.trim_start_matches('/').to_string();

            // If we removed everything, set to empty (will become "root")
            if name_path.is_empty() {
                name_path = "".to_string();
            }
        }
    }

    // Clean the path for name generation (this only affects the name, not the actual URL)
    let clean_path = clean_route_path_for_name(&name_path, config);

    let base_name = if clean_path.is_empty() || clean_path == "/" {
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

    // Use a HashMap to track unique variant names and avoid duplicates
    let mut unique_variants: HashMap<String, &RouteInfo> = HashMap::new();

    for route in routes {
        let variant_name = create_variant_name(&route.name, config);
        let variant_name_str = variant_name.to_string();

        // Check for duplicate variant names
        if let Some(existing_route) = unique_variants.get(&variant_name_str) {
            eprintln!(
                "cargo:warning=Duplicate variant name '{}' for routes: {} {} and {} {}",
                variant_name_str,
                route.method,
                route.path,
                existing_route.method,
                existing_route.path
            );
            continue;
        }

        unique_variants.insert(variant_name_str.clone(), route);
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
    let mut result = match case.to_lowercase().as_str() {
        "pascal" | "pascalcase" => name.to_case(Case::Pascal),
        "camel" | "camelcase" => name.to_case(Case::Camel),
        "snake" | "snake_case" => name.to_case(Case::Snake),
        "kebab" | "kebab-case" => name.to_case(Case::Kebab),
        "title" | "title_case" => name.to_case(Case::Title),
        "lower" | "lowercase" => name.to_lowercase(),
        "upper" | "uppercase" => name.to_uppercase(),
        _ => name.to_case(Case::Pascal), // default
    };

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
    let result = match case.to_lowercase().as_str() {
        "pascal" | "pascalcase" => name.to_case(Case::Pascal),
        "camel" | "camelcase" => name.to_case(Case::Camel),
        "snake" | "snake_case" => name.to_case(Case::Snake),
        "kebab" | "kebab-case" => name.to_case(Case::Kebab),
        "title" | "title_case" => name.to_case(Case::Title),
        "lower" | "lowercase" => name.to_lowercase(),
        "upper" | "uppercase" => name.to_uppercase(),
        _ => name.to_case(Case::Snake), // default
    };
    sanitize_identifier(&result)
}

fn generate_path_build_code(path_template: &str, fields: &[proc_macro2::Ident]) -> TokenStream {
    // Parse the path template and build a sequence of push operations
    let segments: Vec<&str> = path_template.split('/').filter(|s| !s.is_empty()).collect();
    let mut push_operations = Vec::new();

    for (i, segment) in segments.iter().enumerate() {
        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = &segment[1..segment.len() - 1];
            let field_ident = syn::Ident::new(
                &param_name.to_case(Case::Snake),
                proc_macro2::Span::call_site(),
            );

            // Verify this field exists and add it to the path
            if fields.iter().any(|f| f == &field_ident) {
                if i > 0 {
                    push_operations.push(quote! { path.push('/'); });
                }
                push_operations.push(quote! { path.push_str(&#field_ident); });
            } else {
                // Field doesn't exist, use literal
                if i > 0 {
                    push_operations.push(quote! { path.push('/'); });
                }
                push_operations.push(quote! { path.push_str(#segment); });
            }
        } else {
            // Fixed segment
            if i > 0 {
                push_operations.push(quote! { path.push('/'); });
            }
            push_operations.push(quote! { path.push_str(#segment); });
        }
    }

    // Handle empty path (just "/")
    if push_operations.is_empty() {
        quote! { "/".to_string() }
    } else {
        quote! {
            {
                let mut path = String::new();
                #(#push_operations)*
                path
            }
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
