//! TypeScript HTTP client generator.
//!
//! One parameter-descriptor-driven code path replaces the sixteen
//! near-identical branches of the original implementation. Client keys come
//! from the resolved [`NamedRoute`] names, never re-derived from the raw
//! route, so they stay in parity with the generated Rust `name()` values.

use super::super::CodeGenerator;
use super::{TypeImportManager, path_template, ts_param_name};
use crate::configuration::{NamingConfig, TypeScriptConfig};
use crate::error::Result;
use crate::parser::RouteInfo;
use crate::resolved::{NamedRoute, resolve_route_names};
use crate::ts::indent;
use crate::utils::path;
use std::collections::{BTreeMap, BTreeSet};

/// Snippet appended to a client method when the handler takes query params.
const QUERY_STRING_SNIPPET: &str = "\
const queryStr = new URLSearchParams(
  Object.entries(query)
    .filter(([, value]) => value !== undefined && value !== null)
    .map(([key, value]) => [key, String(value)]),
).toString();
if (queryStr) {
  url += `?${queryStr}`;
}";

/// Generates the typed HTTP client plus its API error plumbing.
pub struct ClientGenerator;

impl CodeGenerator for ClientGenerator {
    type Config = TypeScriptConfig;
    type Output = String;

    /// Generates the client, resolving route names under the *default*
    /// naming rules first.
    ///
    /// The TypeScript configuration does not embed the scan-time
    /// [`NamingConfig`], so direct callers without the scan context get the
    /// deterministic names the pipeline produces for the default setup.
    /// Prefer [`ClientGenerator::generate_named`] with a list from
    /// [`resolve_route_names`](crate::resolve_route_names) whenever the
    /// naming configuration is customized.
    fn generate(routes: &[RouteInfo], config: &Self::Config) -> Result<Self::Output> {
        Self::generate_named(
            &resolve_route_names(routes, &NamingConfig::default()),
            config,
        )
    }
}

impl ClientGenerator {
    /// Generates the client from pre-resolved route names.
    ///
    /// Every emitted key equals [`NamedRoute::ts_name`], which is what keeps
    /// the client object, the hooks file, the `OpenAPI` `operationId`s and the
    /// Rust `name()` arms referring to the same identifiers even when the
    /// naming configuration resolves collisions or adds prefix/suffix
    /// decorations.
    ///
    /// # Errors
    ///
    /// Currently infallible; the [`Result`](crate::Result) keeps the
    /// signature uniform with the other generators.
    pub fn generate_named(named: &[NamedRoute<'_>], config: &TypeScriptConfig) -> Result<String> {
        let mut sections: Vec<String> = Vec::new();

        sections.push(imports_section(named, config));
        let interfaces = interfaces_section(named);
        if !interfaces.is_empty() {
            sections.push(interfaces);
        }
        sections.push(runtime_section(config));

        let methods = named
            .iter()
            .map(client_method)
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!(
            "export const client = {{\n{}\n}};",
            indent(&methods, 2)
        ));

        Ok(sections.join("\n\n"))
    }
}

/// Import statements for the client file.
fn imports_section(named: &[NamedRoute<'_>], config: &TypeScriptConfig) -> String {
    let mut lines = vec![
        "import { useQuery, useMutation, type UseQueryOptions, type UseMutationOptions } from \"@tanstack/react-query\";"
            .to_string(),
    ];
    if let Some(import_from) = config.auth_import_from() {
        let token_key = config.auth_token_key().unwrap_or("TOKEN_KEY");
        lines.push(format!("import {{ {token_key} }} from \"{import_from}\";"));
    }
    let mut manager = TypeImportManager::default();
    manager.collect_from_routes(named.iter().map(|entry| entry.route));
    lines.extend(manager.generate_imports(config.bindings_path()));
    lines.join("\n")
}

/// `...Params` interfaces deduplicated by name with sorted fields.
fn interfaces_section(named: &[NamedRoute<'_>]) -> String {
    let mut interfaces: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for entry in named {
        let params: Vec<String> = path::extract_parameters_from_path(&entry.route.path);
        if params.is_empty() {
            continue;
        }
        let fields = params.iter().map(|param| ts_param_name(param));
        interfaces
            .entry(entry.params_interface())
            .or_default()
            .extend(fields);
    }
    interfaces
        .iter()
        .map(|(name, fields)| {
            let field_lines = fields
                .iter()
                .map(|field| format!("  {field}: string;"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("export interface {name} {{\n{field_lines}\n}}")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Shared error types plus the `ApiClient` class and its default instance.
fn runtime_section(config: &TypeScriptConfig) -> String {
    let mut runtime = String::from(RUNTIME_PRELUDE);
    runtime.push('\n');
    runtime.push_str(CLIENT_CLASS);
    runtime.push_str(&api_client_instance(config));
    runtime
}

/// The default `apiClient` instance, with or without auth plumbing.
fn api_client_instance(config: &TypeScriptConfig) -> String {
    match config.auth_import_from() {
        Some(_) => format!(
            "export const apiClient = new ApiClient({{\n  getToken: async () => {{\n    return localStorage.getItem({token_key});\n  }},\n}});\n",
            token_key = config.auth_token_key().unwrap_or("TOKEN_KEY"),
        ),
        None => "export const apiClient = new ApiClient();\n".to_string(),
    }
}

/// The client object entry for a single resolved route.
fn client_method(entry: &NamedRoute<'_>) -> String {
    let route = entry.route;
    let name = entry.ts_name.as_str();
    let params: Vec<String> = path::extract_parameters_from_path(&route.path);
    let has_path_params = !params.is_empty();
    let query_type = route.handler_info.query_params.as_deref();
    let body_type = if route.method == "GET" {
        None
    } else {
        route.handler_info.body_param.as_deref()
    };
    let return_type = route
        .handler_info
        .return_type
        .found_type
        .as_deref()
        .unwrap_or("any");
    let error_type = error_union(route);
    let requires_auth = route.handler_info.requires_auth;

    let mut arguments: Vec<String> = Vec::new();
    if has_path_params {
        arguments.push(format!("params: {}", entry.params_interface()));
    }
    if let Some(query_type) = query_type {
        arguments.push(format!("query: {query_type}"));
    }
    if let Some(body_type) = body_type {
        arguments.push(format!("body: {body_type}"));
    }
    arguments.push("config?: { signal?: AbortSignal }".to_string());

    let mut body_lines: Vec<String> = Vec::new();
    let url_keyword = if query_type.is_some() { "let" } else { "const" };
    body_lines.push(format!(
        "{url_keyword} url = {};",
        path_template(&route.path)
    ));
    if query_type.is_some() {
        for line in QUERY_STRING_SNIPPET.lines() {
            body_lines.push(line.to_string());
        }
    }

    let call = api_call(
        route,
        return_type,
        &error_type,
        requires_auth,
        body_type.is_some(),
    );
    body_lines.push(format!("return {call};"));

    let mut lines = vec![format!(
        "{name}: async ({}): Promise<{return_type}> => {{",
        arguments.join(", ")
    )];
    lines.extend(body_lines);
    lines.push("},".to_string());
    lines.join("\n")
}

/// The `apiClient...` call expression for a route.
fn api_call(
    route: &RouteInfo,
    return_type: &str,
    error_type: &str,
    requires_auth: bool,
    has_body: bool,
) -> String {
    let options = format!("{{ requiresAuth: {requires_auth}, signal: config?.signal }}");
    match route.method.as_str() {
        "GET" => format!("apiClient.get<{return_type}, {error_type}>(url, {options})"),
        "POST" | "PUT" | "PATCH" | "DELETE" => {
            let verb = route.method.to_lowercase();
            let body_argument = if has_body { "body" } else { "undefined" };
            format!(
                "apiClient.{verb}<{return_type}, {error_type}>(url, {body_argument}, {options})"
            )
        }
        other => {
            let body_part = if has_body { "body: body, " } else { "" };
            format!(
                "apiClient.request<{return_type}, {error_type}>(url, {{ method: \"{other}\", {body_part}requiresAuth: {requires_auth}, signal: config?.signal }})"
            )
        }
    }
}

/// `ApiError | ...` union for a route's error types.
fn error_union(route: &RouteInfo) -> String {
    let mut parts = vec!["ApiError".to_string()];
    parts.extend(route.handler_info.return_type.error_types.iter().cloned());
    parts.join(" | ")
}

/// Static error types and the `ApiClient` class emitted with every client.
const RUNTIME_PRELUDE: &str = "\
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
    typeof error === \"object\" &&
    error !== null &&
    \"error\" in error &&
    (error as RawApiError).error === \"Bad Request\" &&
    \"details\" in error &&
    typeof (error as any).details === \"object\" &&
    (error as any).details !== null &&
    \"code\" in (error as any).details &&
    \"message\" in (error as any).details
  );
}
";

/// The `ApiClient` class emitted with every client.
const CLIENT_CLASS: &str = "\
// Base HTTP client with authentication support
class ApiClient {
  private baseUrl: string = \"\";
  private getToken?: () => Promise<string | null>;

  constructor(config?: { baseUrl?: string; getToken?: () => Promise<string | null> }) {
    this.baseUrl = config?.baseUrl || \"\";
    this.getToken = config?.getToken;
  }

  async request<T, E = ApiError>(url: string, options: RequestInit & { requiresAuth?: boolean } = {}): Promise<T> {
    const headers = new Headers(options.headers as Record<string, string>);

    // Set Content-Type for requests with body
    if (options.body && !headers.has(\"Content-Type\")) {
      headers.set(\"Content-Type\", \"application/json\");
    }

    // Add Authorization header if required and token is available
    if (options.requiresAuth && this.getToken) {
      const token = await this.getToken();
      if (token) {
        headers.set(\"Authorization\", \"Bearer \" + token);
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
    if (rawError.error === \"Bad Request\" && rawError.description) {
      try {
        const details = JSON.parse(rawError.description) as BadRequestErrorDetails;
        return {
          ...rawError,
          details,
        };
      } catch (e) {
        // If parsing fails, return the raw error with original description as details
        console.warn(\"Failed to parse error description:\", e);
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
      method: \"GET\",
      requiresAuth: options.requiresAuth,
      signal: options.signal,
    });
  }

  async post<T, E = ApiError>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}): Promise<T> {
    return this.request<T, E>(url, {
      method: \"POST\",
      body: data ? JSON.stringify(data) : undefined,
      requiresAuth: options.requiresAuth,
      signal: options.signal,
    });
  }

  async put<T, E = ApiError>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}): Promise<T> {
    return this.request<T, E>(url, {
      method: \"PUT\",
      body: data ? JSON.stringify(data) : undefined,
      requiresAuth: options.requiresAuth,
      signal: options.signal,
    });
  }

  async patch<T, E = ApiError>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}): Promise<T> {
    return this.request<T, E>(url, {
      method: \"PATCH\",
      body: data ? JSON.stringify(data) : undefined,
      requiresAuth: options.requiresAuth,
      signal: options.signal,
    });
  }

  async delete<T, E = ApiError>(url: string, data?: any, options: { requiresAuth?: boolean; signal?: AbortSignal } = {}): Promise<T> {
    return this.request<T, E>(url, {
      method: \"DELETE\",
      body: data ? JSON.stringify(data) : undefined,
      requiresAuth: options.requiresAuth,
      signal: options.signal,
    });
  }
}
";

#[cfg(test)]
mod tests {
    use super::{
        ClientGenerator, api_client_instance, client_method, error_union, imports_section,
        interfaces_section,
    };
    use crate::configuration::{NamingConfig, TypeScriptConfig};
    use crate::error::Result;
    use crate::generators::CodeGenerator;
    use crate::parser::{HandlerInfo, RouteInfo, handlers::ReturnType};
    use crate::resolved::resolve_route_names;
    use std::collections::BTreeSet;

    use super::super::pascal;

    fn route(path: &str, method: &str, name: &str) -> RouteInfo {
        RouteInfo {
            name: name.to_string(),
            path: path.to_string(),
            method: method.to_string(),
            handler: format!("{name}_handler"),
            handler_info: HandlerInfo::default(),
        }
    }

    /// Resolves the fixture route under the default naming rules.
    fn named(routes: &[RouteInfo]) -> Vec<crate::resolved::NamedRoute<'_>> {
        resolve_route_names(routes, &NamingConfig::default())
    }

    fn rich_route() -> RouteInfo {
        let mut rich = route("/api/users/{user_id}/posts", "GET", "get_user_posts");
        rich.handler_info = HandlerInfo {
            body_param: None,
            query_params: Some("PostQuery".to_string()),
            requires_auth: true,
            return_type: ReturnType {
                found_type: Some("Array<PostResponse>".to_string()),
                is_importable: true,
                error_types: BTreeSet::from(["BadRequest".to_string()]),
                error_variants: BTreeSet::new(),
            },
        };
        rich
    }

    #[test]
    fn generates_client_with_auth_import_and_interfaces() -> Result<()> {
        let config = TypeScriptConfig {
            auth: Some(crate::configuration::AuthConfig {
                token_key_import: Some("AUTH_TOKEN".to_string()),
                import_from: Some("@/lib/auth".to_string()),
            }),
            ..TypeScriptConfig::default()
        };
        let code = ClientGenerator::generate(&[rich_route()], &config)?;
        assert!(code.contains("import { AUTH_TOKEN } from \"@/lib/auth\";"));
        assert!(code.contains("export interface GetUserPostsParams {\n  userId: string;\n}"));
        assert!(code.contains("import { type PostQuery } from \"../../../bindings/PostQuery\";"));
        assert!(
            code.contains("import { type PostResponse } from \"../../../bindings/PostResponse\";")
        );
        assert!(code.contains("localStorage.getItem(AUTH_TOKEN)"));
        assert!(code.contains("getUserPosts: async (params: GetUserPostsParams, query: PostQuery, config?: { signal?: AbortSignal }): Promise<Array<PostResponse>> => {"));
        assert!(code.contains("return apiClient.get<Array<PostResponse>, ApiError | BadRequest>(url, { requiresAuth: true, signal: config?.signal });"));
        Ok(())
    }

    #[test]
    fn anonymous_client_has_no_token_import() {
        let code = ClientGenerator::generate(
            &[route("/health", "GET", "get_health")],
            &TypeScriptConfig::default(),
        )
        .expect("generate");
        assert!(!code.contains("bindings/"));
        assert!(!code.contains("TOKEN_KEY"));
        assert!(code.contains("export const apiClient = new ApiClient();"));
        assert!(
            code.contains(
                "getHealth: async (config?: { signal?: AbortSignal }): Promise<any> => {"
            )
        );
    }

    #[test]
    fn mutations_carry_body_and_use_error_unions() -> Result<()> {
        let mut create = route("/api/users", "POST", "create_user");
        create.handler_info = HandlerInfo {
            body_param: Some("CreateUserRequest".to_string()),
            query_params: None,
            requires_auth: false,
            return_type: ReturnType {
                found_type: Some("UserResponse".to_string()),
                is_importable: true,
                error_types: BTreeSet::from(["BadRequest".to_string()]),
                error_variants: BTreeSet::new(),
            },
        };
        let code = ClientGenerator::generate(&[create], &TypeScriptConfig::default())?;
        assert!(code.contains("createUser: async (body: CreateUserRequest, config?: { signal?: AbortSignal }): Promise<UserResponse> => {"));
        assert!(code.contains("return apiClient.post<UserResponse, ApiError | BadRequest>(url, body, { requiresAuth: false, signal: config?.signal });"));
        Ok(())
    }

    #[test]
    fn unsupported_verbs_fall_back_to_request() {
        let code = ClientGenerator::generate(
            &[route("/things", "HEAD", "head_thing")],
            &TypeScriptConfig::default(),
        )
        .expect("generate");
        assert!(code.contains(
            "return apiClient.request<any, ApiError>(url, { method: \"HEAD\", requiresAuth: false, signal: config?.signal });"
        ));
    }

    #[test]
    fn interfaces_are_sorted_with_sorted_fields() {
        let routes = vec![
            route("/users/{user_id}/x/{account_id}", "GET", "get_user"),
            route("/users/{user_id}/y", "GET", "get_user_alias"),
        ];
        let interfaces = interfaces_section(&named(&routes));
        assert!(interfaces.starts_with("export interface GetUserAliasParams {"));
        assert!(interfaces.contains("export interface GetUserParams {"));
        assert!(interfaces.contains("accountId: string;"));
        assert!(interfaces.contains("userId: string;"));
        let user_section = interfaces
            .split("export interface GetUserParams {")
            .nth(1)
            .expect("interface present");
        assert!(user_section.starts_with("\n  accountId: string;\n  userId: string;"));
    }

    #[test]
    fn imports_skip_auth_when_unconfigured() {
        let routes = vec![route("/x", "GET", "get_x")];
        let imports = imports_section(&named(&routes), &TypeScriptConfig::default());
        assert_eq!(imports.lines().count(), 1);
        assert!(imports.starts_with("import { useQuery"));
    }

    #[test]
    fn api_client_instance_variants() {
        let anonymous = api_client_instance(&TypeScriptConfig::default());
        assert_eq!(anonymous, "export const apiClient = new ApiClient();\n");
        let with_auth = api_client_instance(&TypeScriptConfig {
            auth: Some(crate::configuration::AuthConfig {
                token_key_import: None,
                import_from: Some("@/hooks/use-auth".to_string()),
            }),
            ..TypeScriptConfig::default()
        });
        assert!(with_auth.contains("localStorage.getItem(TOKEN_KEY)"));
    }

    #[test]
    fn error_union_joins_sorted_error_types() {
        let mut subject = route("/x", "GET", "get_x");
        subject.handler_info.return_type.error_types =
            BTreeSet::from(["Zeta".to_string(), "Alpha".to_string()]);
        assert_eq!(error_union(&subject), "ApiError | Alpha | Zeta");
        assert_eq!(error_union(&route("/x", "GET", "get_x")), "ApiError");
    }

    #[test]
    fn client_method_covers_query_and_mutation_bodies() {
        let mut query_route = route("/posts", "GET", "get_posts");
        query_route.handler_info.query_params = Some("PostQuery".to_string());
        let generated = client_method(&named(std::slice::from_ref(&query_route))[0]);
        assert!(generated.contains("let url = `/posts`;"));
        assert!(generated.contains("const queryStr = new URLSearchParams("));
        assert!(generated.contains("url += `?${queryStr}`;"));
        assert!(generated.contains("(query: PostQuery, config?: { signal?: AbortSignal })"));

        let mut delete_route = route("/users/{user_id}", "DELETE", "delete_user");
        delete_route.handler_info.body_param = Some("DeletePayload".to_string());
        let generated = client_method(&named(std::slice::from_ref(&delete_route))[0]);
        assert!(generated.contains(
            "(params: DeleteUserParams, body: DeletePayload, config?: { signal?: AbortSignal })"
        ));
        assert!(generated.contains("return apiClient.delete<any, ApiError>(url, body, { requiresAuth: false, signal: config?.signal });"));
    }

    #[test]
    fn params_interface_and_pascal_helpers() {
        assert_eq!(pascal("get_user"), "GetUser");
        let routes = vec![route("/u/{id}", "GET", "get_u")];
        let named = named(&routes);
        assert_eq!(named[0].params_interface(), "GetUParams");
    }

    #[test]
    fn colliding_routes_produce_unique_client_keys() {
        // Resolved under `append-method`, the POST route surfaces as
        // `postUsers`; both entries must coexist in one client object.
        let routes = vec![
            route("/users", "GET", "users"),
            route("/users", "POST", "users"),
        ];
        let resolution = NamingConfig {
            include_method_in_names: Some(false),
            collision_strategy: Some(crate::configuration::CollisionStrategy::AppendMethod),
            ..NamingConfig::default()
        };
        let named = resolve_route_names(&routes, &resolution);
        let code = ClientGenerator::generate_named(&named, &TypeScriptConfig::default())
            .expect("generate");
        assert_eq!(code.matches("users: async").count(), 1);
        assert!(code.contains("postUsers: async"));
    }
}
