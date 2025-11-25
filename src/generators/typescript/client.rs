use crate::{
    RouteInfo,
    config::TypeScriptConfig,
    generators::{CodeGenerator, typescript::TypeImportManager},
};
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
        let mut interfaces = Vec::new();

        // Initialize type import manager and collect types
        let mut type_manager = TypeImportManager::new();
        type_manager.collect_from_routes(routes);

        // Tanstack query imports
        imports.push(ts_string! {
               import { useQuery, useMutation, type UseQueryOptions, type UseMutationOptions } from "@tanstack/react-query";
           });

        // Expected client imports
        imports.push(ts_string! {
            import { TOKEN_KEY } from "@/hooks/use-auth";
        });

        // Add type imports from the shared manager
        imports.extend(type_manager.generate_imports());

        // Generate the base HTTP client with auth support
        let http_client = generate_http_client();

        for route in routes {
            let method_name = crate::utils::case::convert_to_case(&route.name, "camel");
            let path_params = crate::utils::path::extract_parameters_from_path(&route.path);

            // Generate interfaces for path parameters if needed
            if !path_params.is_empty() {
                let interface = generate_ts_interface(&method_name, "Params", &path_params);
                interfaces.push(interface);
            }

            // Generate client method
            let client_method = generate_client_method(route, &method_name, &path_params);
            client_methods.push(client_method);
        }

        let imports_str = imports.join("\n");
        let interfaces_str = interfaces.join("\n");
        let client_methods_str = client_methods.join("\n");

        // Combine all parts
        let ts_code = ts_string! {
            #imports_str
            #interfaces_str

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

fn generate_client_method(route: &RouteInfo, method_name: &str, path_params: &[String]) -> String {
    let _method_upper = route.method.to_uppercase();
    let path_template = generate_ts_path_template(&route.path, path_params);

    let body_type = route.handler_info.body_param.as_deref().unwrap_or("void");
    let query_type = route.handler_info.query_params.as_deref().unwrap_or("void");
    let return_type = route
        .handler_info
        .return_type
        .found_type
        .as_deref()
        .unwrap_or("any");
    let requires_auth = route.handler_info.requires_auth;

    // Generate error union for this specific method
    let error_union = generate_route_error_union(route);

    let has_path_params = !path_params.is_empty();
    let has_query_params = route.handler_info.query_params.is_some();
    let has_body = route.method != "GET" && body_type != "void";

    if route.method == "GET" {
        // GET request with various parameter combinations
        if !has_path_params && !has_query_params {
            // No parameters
            ts_string! {
                #method_name: async (config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.get<#return_type, #error_union>(url, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else if has_path_params && !has_query_params {
            // Only path parameters
            let params_type = format!(
                "{}Params",
                crate::utils::case::convert_to_case(method_name, "pascal")
            );
            ts_string! {
                #method_name: async (params: #params_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.get<#return_type, #error_union>(url, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else if !has_path_params && has_query_params {
            // Only query parameters
            ts_string! {
                #method_name: async (query: #query_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    let url = #path_template;
                    const queryString = new URLSearchParams();
                    Object.entries(query).forEach(([key, value]) => {
                        if (value !== undefined && value !== null) {
                            queryString.append(key, value.toString());
                        }
                    });
                    const queryStr = queryString.toString();
                    if (queryStr) {
                        url += "?"+queryStr;
                    }
                    return apiClient.get<#return_type, #error_union>(url, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else {
            // Both path and query parameters
            let params_type = format!(
                "{}Params",
                crate::utils::case::convert_to_case(method_name, "pascal")
            );
            ts_string! {
                #method_name: async (params: #params_type, query: #query_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    let url = #path_template;
                    const queryString = new URLSearchParams();
                    Object.entries(query).forEach(([key, value]) => {
                        if (value !== undefined && value !== null) {
                            queryString.append(key, value.toString());
                        }
                    });
                    const queryStr = queryString.toString();
                    if (queryStr) {
                        url += "?"+queryStr;
                    }
                    return apiClient.get<#return_type, #error_union>(url, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        }
    } else {
        // Non-GET requests (POST, PUT, PATCH, DELETE)
        let method_call = match route.method.as_str() {
            "POST" => "post",
            "PUT" => "put",
            "PATCH" => "patch",
            "DELETE" => "delete",
            _ => "post",
        };

        if !has_path_params && !has_query_params && !has_body {
            // No parameters at all
            ts_string! {
                #method_name: async (config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.#method_call<#return_type, #error_union>(url, undefined, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else if has_path_params && !has_query_params && !has_body {
            // Only path parameters, no body
            let params_type = format!(
                "{}Params",
                crate::utils::case::convert_to_case(method_name, "pascal")
            );
            ts_string! {
                #method_name: async (params: #params_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.#method_call<#return_type, #error_union>(url, undefined, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else if !has_path_params && has_query_params && !has_body {
            // Only query parameters, no body
            ts_string! {
                #method_name: async (query: #query_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    let url = #path_template;
                    const queryString = new URLSearchParams();
                    Object.entries(query).forEach(([key, value]) => {
                        if (value !== undefined && value !== null) {
                            queryString.append(key, value.toString());
                        }
                    });
                    const queryStr = queryString.toString();
                    if (queryStr) {
                        url += "?"+queryStr;
                    }
                    return apiClient.#method_call<#return_type, #error_union>(url, undefined, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else if has_path_params && has_query_params && !has_body {
            // Path and query parameters, no body
            let params_type = format!(
                "{}Params",
                crate::utils::case::convert_to_case(method_name, "pascal")
            );
            ts_string! {
                #method_name: async (params: #params_type, query: #query_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    let url = #path_template;
                    const queryString = new URLSearchParams();
                    Object.entries(query).forEach(([key, value]) => {
                        if (value !== undefined && value !== null) {
                            queryString.append(key, value.toString());
                        }
                    });
                    const queryStr = queryString.toString();
                    if (queryStr) {
                        url += "?"+queryStr;
                    }
                    return apiClient.#method_call<#return_type, #error_union>(url, undefined, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else if !has_path_params && !has_query_params && has_body {
            // Only body
            ts_string! {
                #method_name: async (body: #body_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.#method_call<#return_type, #error_union>(url, body, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else if has_path_params && !has_query_params && has_body {
            // Path parameters and body
            let params_type = format!(
                "{}Params",
                crate::utils::case::convert_to_case(method_name, "pascal")
            );
            ts_string! {
                #method_name: async (params: #params_type, body: #body_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    const url = #path_template;
                    return apiClient.#method_call<#return_type, #error_union>(url, body, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else if !has_path_params && has_query_params && has_body {
            // Query parameters and body
            ts_string! {
                #method_name: async (query: #query_type, body: #body_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    let url = #path_template;
                    const queryString = new URLSearchParams();
                    Object.entries(query).forEach(([key, value]) => {
                        if (value !== undefined && value !== null) {
                            queryString.append(key, value.toString());
                        }
                    });
                    const queryStr = queryString.toString();
                    if (queryStr) {
                        url += "?"+queryStr;
                    }
                    return apiClient.#method_call<#return_type, #error_union>(url, body, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        } else {
            // All three: path parameters, query parameters, and body
            let params_type = format!(
                "{}Params",
                crate::utils::case::convert_to_case(method_name, "pascal")
            );
            ts_string! {
                #method_name: async (params: #params_type, query: #query_type, body: #body_type, config?: { signal?: AbortSignal }): Promise<#return_type> => {
                    let url = #path_template;
                    const queryString = new URLSearchParams();
                    Object.entries(query).forEach(([key, value]) => {
                        if (value !== undefined && value !== null) {
                            queryString.append(key, value.toString());
                        }
                    });
                    const queryStr = queryString.toString();
                    if (queryStr) {
                        url += "?"+queryStr;
                    }
                    return apiClient.#method_call<#return_type, #error_union>(url, body, { requiresAuth: #requires_auth, signal: config?.signal });
                },
            }
        }
    }.to_string()
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
    // ... (keep the existing http client implementation unchanged)
    // This function remains the same as in your original code
    ts_string! {
        // Base error type that comes from the server
        export type RawApiError = {
            error: string;
            description: string;
        };

        // Parsed error type with structured details
        export type ApiError<TDetails = unknown> = RawApiError & {
            details: TDetails;
        };

        // Common error details structure for Bad Request errors
        export type BadRequestErrorDetails = {
            code: string;
            message: string;
        };

        // Type guard to check if error is a Bad Request with structured details
        export function isBadRequestError(error: unknown): error is ApiError<BadRequestErrorDetails> {
            return (
                typeof error === "object" &&
                error !== null &&
                "error" in error &&
                (error as RawApiError).error === "Bad Request" &&
                "details" in error &&
                typeof (error as any).details === "object" &&
                (error as any).details !== null &&
                "code" in (error as any).details &&
                "message" in (error as any).details
            );
        }

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
                    const rawError = await response.json() as RawApiError;

                    // Transform the error to include parsed details
                    const transformedError = this.transformError(rawError);
                    throw transformedError;
                }

                // For 204 No Content responses, return null
                if (response.status === 204) {
                    return null;
                }

                return response.json() as Promise<T>;
            }

            private transformError(rawError: RawApiError): ApiError {
                // For Bad Request errors, parse the description field
                if (rawError.error === "Bad Request" && rawError.description) {
                    try {
                        const details = JSON.parse(rawError.description) as BadRequestErrorDetails;
                        return {
                            ...rawError,
                            details,
                        };
                    } catch (e) {
                        // If parsing fails, return the raw error with original description as details
                        console.warn("Failed to parse error description:", e);
                        return {
                            ...rawError,
                            details: rawError.description,
                        };
                    }
                }

                // For other error types, use the description as details
                return {
                    ...rawError,
                    details: rawError.description,
                };
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

fn generate_ts_interface(method_name: &str, suffix: &str, params: &[String]) -> String {
    let interface_name = format!(
        "{}{}",
        crate::utils::case::convert_to_case(method_name, "pascal"),
        suffix
    );
    let mut fields = Vec::new();

    for param in params {
        let field_name = crate::utils::case::convert_to_case(param, "camel");
        fields.push(ts_string! {
            #field_name: string;
        });
    }
    let fields_str = fields.join("\n");
    ts_string! {
        export interface #interface_name {
            #fields_str
        }
    }
}
