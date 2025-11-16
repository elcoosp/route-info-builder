use super::super::CodeGenerator;
use crate::{RouteInfo, config::TypeScriptConfig};
use ts_quote::ts_string;

pub struct TypeScriptHooksGenerator;

impl CodeGenerator for TypeScriptHooksGenerator {
    type Config = TypeScriptConfig;
    type Output = String;

    fn generate(
        routes: &[RouteInfo],
        _config: &Self::Config,
    ) -> Result<Self::Output, Box<dyn std::error::Error>> {
        let mut hooks = Vec::new();
        let mut interfaces = Vec::new();

        for route in routes {
            let method_name = crate::utils::case::convert_to_case(&route.name, "camel");
            let hook_name = format!(
                "use{}",
                crate::utils::case::convert_to_case(&route.name, "pascal")
            );
            let params = crate::utils::path::extract_parameters_from_path(&route.path);

            // Generate parameter interface if needed
            if !params.is_empty() {
                let interface = generate_ts_interface(&method_name, &params);
                interfaces.push(interface);
            }

            // Generate hook
            let hook = generate_ts_hook(route, &method_name, &hook_name, &params);
            hooks.push(hook);
        }

        let interfaces_str = interfaces.join("\n");
        let hooks_str = hooks.join("\n");

        // Combine all parts
        let ts_code = ts_string! {
            // Interfaces
            #interfaces_str

            // Hooks
            #hooks_str
        };

        // Format the TypeScript code
        let formatted = super::format_ts_code(&ts_code.to_string())?;
        Ok(formatted)
    }
}

fn generate_ts_interface(method_name: &str, params: &[String]) -> String {
    let interface_name = format!(
        "{}Params",
        crate::utils::case::convert_to_case(method_name, "pascal")
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
    let return_type = route
        .handler_info
        .return_type
        .found_type
        .as_deref()
        .unwrap_or("any");
    let _requires_auth = route.handler_info.requires_auth;

    if route.method == "GET" {
        if params.is_empty() {
            ts_string! {
                export function #hook_name(options?: Omit<UseQueryOptions<#return_type, ApiError>, "queryKey">) {
                    return useQuery({
                        queryKey: [#method_name_str],
                        queryFn: ({ signal }) => client.#method_name({ signal }),
                        ...options,
                    });
                }
            }
            .into()
        } else {
            let params_type = format!(
                "{}Params",
                crate::utils::case::convert_to_case(method_name, "pascal")
            );
            ts_string! {
                export function #hook_name(params: #params_type, options?: Omit<UseQueryOptions<#return_type, ApiError>, "queryKey">) {
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
                export function #hook_name(options?: UseMutationOptions<#return_type, ApiError, #body_type, unknown>) {
                    return useMutation({
                        mutationFn: (body: #body_type) => client.#method_name(body),
                        ...options,
                    });
                }
            }
            .into()
        } else {
            let params_type = format!(
                "{}Params",
                crate::utils::case::convert_to_case(method_name, "pascal")
            );
            ts_string! {
                export function #hook_name(options?: UseMutationOptions<#return_type, ApiError, { params: #params_type, body: #body_type }, unknown>) {
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
