use super::super::CodeGenerator;
use crate::{RouteInfo, config::TypeScriptConfig};
use std::collections::HashSet;
use ts_quote::ts_string;

pub struct TypeScriptClientGenerator;

impl CodeGenerator for TypeScriptClientGenerator {
    type Config = TypeScriptConfig;
    type Output = String;

    fn generate(
        routes: &[RouteInfo],
        _config: &Self::Config,
    ) -> Result<Self::Output, Box<dyn std::error::Error>> {
        let mut imports = Vec::new();
        let mut client_methods = Vec::new();
        let mut type_imports = HashSet::new();
        let mut error_imports = HashSet::new();

        // Tanstack query imports
        imports.push(ts_string! {
            import { useQuery, useMutation, type UseQueryOptions, type UseMutationOptions } from "@tanstack/react-query";
        });

        // Expected client imports
        imports.push(ts_string! {
            import { TOKEN_KEY } from "@/hooks/use-auth";
        });

        // Collect all unique body types, return types, and error types for import
        for route in routes {
            // Handle body types
            if let Some(body_type) = &route.handler_info.body_param {
                extract_importable_types(body_type, &mut type_imports);
            }

            // Handle return types
            if let Some(return_type) = &route.handler_info.return_type.found_type
                && route.handler_info.return_type.is_importable {
                    extract_importable_types(return_type, &mut type_imports);
                }

            // Handle error types
            for error_type in &route.handler_info.return_type.error_types {
                error_imports.insert(error_type.clone());
            }
        }

        // Generate type imports
        for type_name in &type_imports {
            // FIXME get path from config
            let import = format!("\"../../../bindings/{type_name}\"");
            imports.push(ts_string! {
                import { type #type_name } from #import;
            });
        }

        // Generate error type imports
        if !error_imports.is_empty() {
            let error_imports_vec: Vec<String> = error_imports.into_iter().collect();
            let error_imports_str = error_imports_vec.join(", ");

            // FIXME: Get path from config
            let import_path = "\"../../../bindings\"";
            imports.push(ts_string! {
                import { #error_imports_str } from #import_path;
            });
        }

        // Generate the base HTTP client with auth support
        let http_client = generate_http_client();

        for route in routes {
            let method_name = crate::utils::case::convert_to_case(&route.name, "camel");
            let params = crate::utils::path::extract_parameters_from_path(&route.path);

            // Generate client method
            let client_method = generate_client_method(route, &method_name, &params);
            client_methods.push(client_method);
        }

        let imports_str = imports.join("\n");
        let client_methods_str = client_methods.join("\n");

        // Combine all parts
        let ts_code = ts_string! {
            #imports_str

            // HTTP client with auth support
            #http_client

            // Client
            export const client = {
                #client_methods_str
            };
        };

        // Format the TypeScript code
        let formatted = super::format_ts_code(&ts_code.to_string())?;
        Ok(formatted)
    }
}

fn generate_client_method(route: &RouteInfo, method_name: &str, params: &[String]) -> String {
    let _method_upper = route.method.to_uppercase();
    let path_template = generate_ts_path_template(&route.path, params);

    let body_type = route.handler_info.body_param.as_deref().unwrap_or("void");
    let return_type = route
        .handler_info
        .return_type
        .found_type
        .as_deref()
        .unwrap_or("any");
    let requires_auth = route.handler_info.requires_auth;

    // Generate error union for this specific method
    let error_union = generate_route_error_union(route);

    if params.is_empty() {
        if route.method == "GET" {
            ts_string! {
                #method_name: async (config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.get<#return_type, #error_union>(url, { requiresAuth: #requires_auth, signal: config?.signal });
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
                    return apiClient.#method_call<#return_type, #error_union>(url, body, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        }
    } else {
        let params_type = format!(
            "{}Params",
            crate::utils::case::convert_to_case(method_name, "pascal")
        );

        if route.method == "GET" {
            ts_string! {
                #method_name: async (params: #params_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.get<#return_type, #error_union>(url, { requiresAuth: #requires_auth, signal: config?.signal });
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
                    return apiClient.#method_call<#return_type, #error_union>(url, body, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        }
    }
}

/// Generate error union type for a specific route
fn generate_route_error_union(route: &RouteInfo) -> String {
    let mut error_types = vec!["ApiError".to_string()];

    // Add custom error types from the handler
    for error_type in &route.handler_info.return_type.error_types {
        error_types.push(error_type.clone());
    }

    error_types.join(" | ")
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
            let ts_param_name = crate::utils::case::convert_to_case(param_name, "camel");
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

fn generate_http_client() -> String {
    ts_string! {
        export type ApiError = {
            error: string;
            description: string;
        };

        // Base HTTP client with authentication support
        class ApiClient {
            private baseUrl: string = "";
            private getToken?: () => Promise<string | null>;

            constructor(config?: { baseUrl?: string; getToken?: () => Promise<string | null> }) {
                this.baseUrl = config?.baseUrl || "";
                this.getToken = config?.getToken;
            }

            async request<T, E = ApiError>(url: string, options: RequestInit & { requiresAuth?: boolean } = {}): Promise<T> {
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

                const response = await fetch(this.baseUrl + url, {
                    ...options,
                    headers,
                });

                if (!response.ok) {
                    throw (await response.json() as E);
                }

                // For 204 No Content responses, return null
                if (response.status === 204) {
                    return null as T;
                }

                return response.json() as Promise<T>;
            }

            async get<T, E = ApiError>(url: string, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}): Promise<T> {
                return this.request<T, E>(url, {
                    method: "GET",
                    requiresAuth: options.requiresAuth,
                    signal: options.signal,
                });
            }

            async post<T, E = ApiError>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}): Promise<T> {
                return this.request<T, E>(url, {
                    method: "POST",
                    body: data ? JSON.stringify(data) : undefined,
                    requiresAuth: options.requiresAuth,
                    signal: options.signal,
                });
            }

            async put<T, E = ApiError>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}): Promise<T> {
                return this.request<T, E>(url, {
                    method: "PUT",
                    body: data ? JSON.stringify(data) : undefined,
                    requiresAuth: options.requiresAuth,
                    signal: options.signal,
                });
            }

            async patch<T, E = ApiError>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}): Promise<T> {
                return this.request<T, E>(url, {
                    method: "PATCH",
                    body: data ? JSON.stringify(data) : undefined,
                    requiresAuth: options.requiresAuth,
                    signal: options.signal,
                });
            }

            async delete<T, E = ApiError>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}): Promise<T> {
                return this.request<T, E>(url, {
                    method: "DELETE",
                    body: data ? JSON.stringify(data) : undefined,
                    requiresAuth: options.requiresAuth,
                    signal: options.signal,
                });
            }
        }

        // Create default instance
        export const apiClient = new ApiClient({
          getToken: async () => {
            return localStorage.getItem(TOKEN_KEY);
          },
        });
    }
}

/// Extract importable types from type strings, handling generics like Array<T>
fn extract_importable_types(type_str: &str, imports: &mut HashSet<String>) {
    // Check if this is a generic type like Array<T>
    if let Some(inner_type) = extract_generic_inner_type(type_str) {
        // Recursively extract inner types (for nested generics)
        extract_importable_types(&inner_type, imports);
    } else if !is_builtin_type(type_str) {
        // Only add non-builtin types
        imports.insert(type_str.to_string());
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
