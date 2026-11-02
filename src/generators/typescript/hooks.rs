//! `TanStack` Query hooks generator.
//!
//! Mirrors the client generator: a single parameter-descriptor-driven code
//! path builds `useQuery` hooks for GET routes and `useMutation` hooks for
//! everything else.

use super::super::CodeGenerator;
use super::{TypeImportManager, method_display_name, params_interface_name, pascal};
use crate::configuration::TypeScriptConfig;
use crate::error::Result;
use crate::parser::RouteInfo;
use std::collections::BTreeSet;

/// Generates `useQuery`/`useMutation` hooks delegating to the client file.
pub struct HooksGenerator;

impl CodeGenerator for HooksGenerator {
    type Config = TypeScriptConfig;
    type Output = String;

    fn generate(routes: &[RouteInfo], config: &Self::Config) -> Result<Self::Output> {
        let mut sections: Vec<String> = Vec::new();
        sections.push(imports_section(routes, config));

        let hooks = routes
            .iter()
            .map(hook_function)
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(hooks);

        Ok(sections.join("\n\n"))
    }
}

/// Import statements plus the convenience re-exports for the hooks file.
fn imports_section(routes: &[RouteInfo], config: &TypeScriptConfig) -> String {
    let mut lines = vec![
        "import { type ApiError, type BadRequestErrorDetails, isBadRequestError, client } from \"./client\";"
            .to_string(),
        "import { useQuery, useMutation, type UseQueryOptions, type UseMutationOptions } from \"@tanstack/react-query\";"
            .to_string(),
    ];

    let mut manager = TypeImportManager::default();
    manager.collect_from_routes(routes);
    lines.extend(manager.generate_imports(config.bindings_path()));

    let mut client_type_imports: BTreeSet<String> = BTreeSet::new();
    for route in routes {
        if !crate::utils::path::extract_parameters_from_path(&route.path).is_empty() {
            client_type_imports.insert(format!("type {}", params_interface_name(route)));
        }
    }
    if !client_type_imports.is_empty() {
        let joined = client_type_imports
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("import {{ {joined} }} from \"./client\";"));
    }

    lines.push(
        "// Re-export error utilities for convenience\nexport { type ApiError, type BadRequestErrorDetails, isBadRequestError };"
            .to_string(),
    );
    lines.join("\n")
}

/// The hook function for a single route.
fn hook_function(route: &RouteInfo) -> String {
    let hook_name = format!("use{}", pascal(&route.name));
    let method_name = method_display_name(route);
    let return_type = route
        .handler_info
        .return_type
        .found_type
        .as_deref()
        .unwrap_or("any");
    let params_type = params_interface_name(route);
    let query_type = route.handler_info.query_params.as_deref();
    let body_type = if route.method == "GET" {
        None
    } else {
        route.handler_info.body_param.as_deref()
    };

    if route.method == "GET" {
        query_hook(
            &hook_name,
            &method_name,
            return_type,
            has_path_params(route),
            &params_type,
            query_type,
        )
    } else {
        mutation_hook(
            &hook_name,
            &method_name,
            return_type,
            has_path_params(route),
            &params_type,
            query_type,
            body_type,
        )
    }
}

/// Whether the route path declares parameters.
fn has_path_params(route: &RouteInfo) -> bool {
    !crate::utils::path::extract_parameters_from_path(&route.path).is_empty()
}

/// Builds a `useQuery` hook for a GET route.
fn query_hook(
    hook_name: &str,
    method_name: &str,
    return_type: &str,
    has_params: bool,
    params_type: &str,
    query_type: Option<&str>,
) -> String {
    let mut signature: Vec<String> = Vec::new();
    let mut query_key_items = vec![format!("\"{method_name}\"")];
    let mut client_arguments: Vec<String> = Vec::new();

    if has_params {
        signature.push(format!("params: {params_type}"));
        query_key_items.push("params".to_string());
        client_arguments.push("params".to_string());
    }
    if let Some(query_type) = query_type {
        signature.push(format!("query: {query_type}"));
        query_key_items.push("query".to_string());
        client_arguments.push("query".to_string());
    }
    signature.push(format!(
        "options?: Omit<UseQueryOptions<{return_type}, ApiError>, \"queryKey\">"
    ));

    let signature = signature.join(", ");
    let key = query_key_items.join(", ");
    let client_arguments = client_arguments.join(", ");
    let signal_argument = if client_arguments.is_empty() {
        "{ signal }".to_string()
    } else {
        format!("{client_arguments}, {{ signal }}")
    };
    format!(
        "export function {hook_name}({signature}) {{\n  return useQuery({{\n    queryKey: [{key}],\n    queryFn: ({{ signal }}) => client.{method_name}({signal_argument}),\n    ...options,\n  }});\n}}"
    )
}

/// Builds a `useMutation` hook for a non-GET route.
fn mutation_hook(
    hook_name: &str,
    method_name: &str,
    return_type: &str,
    has_params: bool,
    params_type: &str,
    query_type: Option<&str>,
    body_type: Option<&str>,
) -> String {
    let mut variables: Vec<(&str, String)> = Vec::new();
    if has_params {
        variables.push(("params", params_type.to_string()));
    }
    if let Some(query_type) = query_type {
        variables.push(("query", query_type.to_string()));
    }
    if let Some(body_type) = body_type {
        variables.push(("body", body_type.to_string()));
    }

    let variables_type = match variables.len() {
        0 => "void".to_string(),
        1 => variables[0].1.clone(),
        _ => {
            let fields = variables
                .iter()
                .map(|(name, var_type)| format!("{name}: {var_type}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
    };

    let (mutation_argument, client_arguments) = match variables.len() {
        0 => (String::new(), String::new()),
        1 => {
            let (name, var_type) = &variables[0];
            (format!("{name}: {var_type}"), name.to_string())
        }
        _ => {
            let names = variables
                .iter()
                .map(|(name, _)| format!("input.{name}"))
                .collect::<Vec<_>>()
                .join(", ");
            ("input: OBJECT".to_string(), names)
        }
    };
    let mutation_argument = mutation_argument.replace("OBJECT", &variables_type);

    format!(
        "export function {hook_name}(options?: UseMutationOptions<{return_type}, ApiError, {variables_type}, unknown>) {{\n  return useMutation({{\n    mutationFn: ({mutation_argument}) => client.{method_name}({client_arguments}),\n    ...options,\n  }});\n}}"
    )
}

#[cfg(test)]
mod tests {
    use super::{HooksGenerator, hook_function, imports_section, mutation_hook, query_hook};
    use crate::configuration::TypeScriptConfig;
    use crate::error::Result;
    use crate::generators::CodeGenerator;
    use crate::parser::{HandlerInfo, RouteInfo, handlers::ReturnType};
    use std::collections::BTreeSet;

    fn route(path: &str, method: &str, name: &str) -> RouteInfo {
        RouteInfo {
            name: name.to_string(),
            path: path.to_string(),
            method: method.to_string(),
            handler: format!("{name}_handler"),
            handler_info: HandlerInfo::default(),
        }
    }

    #[test]
    fn get_hook_without_params() {
        let hook = hook_function(&route("/health", "GET", "get_health"));
        assert_eq!(
            hook,
            "export function useGetHealth(options?: Omit<UseQueryOptions<any, ApiError>, \"queryKey\">) {\n  return useQuery({\n    queryKey: [\"getHealth\"],\n    queryFn: ({ signal }) => client.getHealth({ signal }),\n    ...options,\n  });\n}"
        );
    }

    #[test]
    fn get_hook_with_params_and_query() {
        let mut rich = route("/users/{user_id}/feed", "GET", "get_user_feed");
        rich.handler_info.query_params = Some("FeedQuery".to_string());
        let hook = hook_function(&rich);
        assert!(hook.starts_with("export function useGetUserFeed(params: GetUserFeedParams, query: FeedQuery, options?: Omit<UseQueryOptions<any, ApiError>, \"queryKey\">) {"));
        assert!(hook.contains("queryKey: [\"getUserFeed\", params, query],"));
        assert!(
            hook.contains(
                "queryFn: ({ signal }) => client.getUserFeed(params, query, { signal }),"
            )
        );
    }

    #[test]
    fn mutation_hooks_cover_all_variable_shapes() {
        let plain = hook_function(&route("/health", "POST", "post_health"));
        assert!(plain.contains("UseMutationOptions<any, ApiError, void, unknown>"));
        assert!(plain.contains("mutationFn: () => client.postHealth(),"));

        let mut with_body = route("/users", "POST", "create_user");
        with_body.handler_info.body_param = Some("CreateUserRequest".to_string());
        let hook = hook_function(&with_body);
        assert!(hook.contains("UseMutationOptions<any, ApiError, CreateUserRequest, unknown>"));
        assert!(hook.contains("mutationFn: (body: CreateUserRequest) => client.createUser(body),"));

        let mut with_query = route("/users", "POST", "post_user");
        with_query.handler_info.query_params = Some("UserQuery".to_string());
        let hook = hook_function(&with_query);
        assert!(hook.contains("UseMutationOptions<any, ApiError, UserQuery, unknown>"));
        assert!(hook.contains("mutationFn: (query: UserQuery) => client.postUser(query),"));

        let mut with_params = route("/users/{user_id}", "PUT", "put_user");
        with_params.handler_info.body_param = Some("UpdateUserRequest".to_string());
        let hook = hook_function(&with_params);
        assert!(hook.contains(
            "UseMutationOptions<any, ApiError, { params: PutUserParams, body: UpdateUserRequest }, unknown>"
        ));
        assert!(hook.contains(
            "mutationFn: (input: { params: PutUserParams, body: UpdateUserRequest }) => client.putUser(input.params, input.body),"
        ));

        let mut with_all = route("/users/{user_id}/settings", "PATCH", "patch_user_settings");
        with_all.handler_info.query_params = Some("SettingsQuery".to_string());
        with_all.handler_info.body_param = Some("SettingsRequest".to_string());
        let hook = hook_function(&with_all);
        assert!(hook.contains(
            "UseMutationOptions<any, ApiError, { params: PatchUserSettingsParams, query: SettingsQuery, body: SettingsRequest }, unknown>"
        ));
        assert!(hook.contains(
            "mutationFn: (input: { params: PatchUserSettingsParams, query: SettingsQuery, body: SettingsRequest }) => client.patchUserSettings(input.params, input.query, input.body),"
        ));
    }

    #[test]
    fn hooks_file_imports_client_types() -> Result<()> {
        let mut rich = route("/users/{user_id}", "GET", "get_user");
        rich.handler_info.query_params = Some("UserQuery".to_string());
        rich.handler_info.return_type = ReturnType {
            found_type: Some("UserResponse".to_string()),
            is_importable: true,
            error_types: BTreeSet::from(["BadRequest".to_string()]),
            error_variants: BTreeSet::new(),
        };
        let code = HooksGenerator::generate(&[rich], &TypeScriptConfig::default())?;
        assert!(code.contains("import { type ApiError, type BadRequestErrorDetails, isBadRequestError, client } from \"./client\";"));
        assert!(
            code.contains("import { type UserResponse } from \"../../../bindings/UserResponse\";")
        );
        assert!(code.contains("import { type BadRequest } from \"../../../bindings/BadRequest\";"));
        assert!(code.contains("import { type GetUserParams } from \"./client\";"));
        assert!(code.contains("import { type UserQuery } from \"../../../bindings/UserQuery\";"));
        assert!(
            code.contains(
                "export { type ApiError, type BadRequestErrorDetails, isBadRequestError };"
            )
        );
        Ok(())
    }

    #[test]
    fn imports_without_extra_types_stay_minimal() {
        let imports = imports_section(
            &[route("/health", "GET", "get_health")],
            &TypeScriptConfig::default(),
        );
        assert!(!imports.contains("from \"./client\";\nimport { type"));
        assert_eq!(imports.lines().count(), 4);
    }

    #[test]
    fn helper_branches_are_exercised() {
        // Ensure the unused-parameter branches of the builders stay covered.
        let query = query_hook("useX", "getX", "Resp", false, "XParams", None);
        assert!(query.contains("queryKey: [\"getX\"]"));
        let mutation = mutation_hook("useY", "postY", "Resp", false, "YParams", None, None);
        assert!(mutation.contains("UseMutationOptions<Resp, ApiError, void, unknown>"));
    }
}
