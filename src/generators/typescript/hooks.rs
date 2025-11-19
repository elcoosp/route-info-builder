use crate::{
    RouteInfo,
    config::TypeScriptConfig,
    generators::{CodeGenerator, typescript::TypeImportManager},
};
use ts_quote::ts_string;

pub struct TypeScriptHooksGenerator;

impl CodeGenerator for TypeScriptHooksGenerator {
    type Config = TypeScriptConfig;
    type Output = String;

    fn generate(
        routes: &[RouteInfo],
        _config: &Self::Config,
    ) -> Result<Self::Output, Box<dyn std::error::Error>> {
        let mut imports = Vec::new();
        let mut hooks = Vec::new();
        let mut client_imports = Vec::new();

        // Initialize type import manager and collect types
        let mut type_manager = TypeImportManager::new();
        type_manager.collect_from_routes(routes);

        // Import necessary types from the client
        imports.push(ts_string! {
               import { type ApiError, type BadRequestErrorDetails, isBadRequestError, client } from "./client";
           });

        // Tanstack query imports
        imports.push(ts_string! {
               import { useQuery, useMutation, type UseQueryOptions, type UseMutationOptions } from "@tanstack/react-query";
           });

        // Add type imports from the shared manager
        imports.extend(type_manager.generate_imports());

        for route in routes {
            let method_name = crate::utils::case::convert_to_case(&route.name, "camel");
            let hook_name = format!(
                "use{}",
                crate::utils::case::convert_to_case(&route.name, "pascal")
            );
            let params = crate::utils::path::extract_parameters_from_path(&route.path);
            if !params.is_empty() {
                let interface_name = format!(
                    "{}Params",
                    crate::utils::case::convert_to_case(&method_name, "pascal")
                );
                client_imports.push(format!("type {interface_name}"));
            }
            // Generate hook with proper error union type
            let hook = generate_ts_hook(route, &method_name, &hook_name, &params);
            hooks.push(hook);
        }

        if !client_imports.is_empty() {
            let client_imports_str = client_imports.join(", ");
            imports.push(ts_string! {
                import { #client_imports_str } from "./client";
            });
        }

        let imports_str = imports.join("\n");
        let hooks_str = hooks.join("\n");

        // Combine all parts
        let ts_code = ts_string! {
            #imports_str

            // Re-export error utilities for convenience
            export { type ApiError, type BadRequestErrorDetails, isBadRequestError };

            // Hooks
            #hooks_str
        };

        // Format the TypeScript code
        let formatted = super::format_ts_code(&ts_code.to_string())?;
        Ok(formatted)
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
    let return_type = route
        .handler_info
        .return_type
        .found_type
        .as_deref()
        .unwrap_or("any");
    let _requires_auth = route.handler_info.requires_auth;

    // All hooks now use ApiError as the error type
    let error_type = "ApiError";

    if route.method == "GET" {
        if params.is_empty() {
            ts_string! {
                export function #hook_name(options?: Omit<UseQueryOptions<#return_type, #error_type>, "queryKey">) {
                    return useQuery({
                        queryKey: [#method_name_str],
                        queryFn: ({ signal }) => client.#method_name({ signal }),
                        ...options,
                    });
                }
            }
        } else {
            let params_type = format!(
                "{}Params",
                crate::utils::case::convert_to_case(method_name, "pascal")
            );
            ts_string! {
                export function #hook_name(params: #params_type, options?: Omit<UseQueryOptions<#return_type, #error_type>, "queryKey">) {
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
                export function #hook_name(options?: UseMutationOptions<#return_type, #error_type, #body_type, unknown>) {
                    return useMutation({
                        mutationFn: (body: #body_type) => client.#method_name(body),
                        ...options,
                    });
                }
            }
        } else {
            let params_type = format!(
                "{}Params",
                crate::utils::case::convert_to_case(method_name, "pascal")
            );
            ts_string! {
                export function #hook_name(options?: UseMutationOptions<#return_type, #error_type, { params: #params_type, body: #body_type }, unknown>) {
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
