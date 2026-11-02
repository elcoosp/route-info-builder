//! End-to-end tests exercising the public API against fixture controllers.

use std::path::Path;

use route_info_builder::{
    Config, RustConfig, TypeScriptConfig, generate_links, generate_openapi_json,
    generate_route_map_json, generate_ts_client, generate_ts_hooks, scan_routes,
};

/// Writes the fixture controller tree used by most tests.
fn write_fixtures(dir: &Path) -> std::io::Result<()> {
    let controllers = dir.join("controllers");
    std::fs::create_dir_all(controllers.join("admin"))?;

    std::fs::write(
        controllers.join("users.rs"),
        r#"
use axum::extract::{Json, Query};
use crate::error::AppError;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/users")
        .add("/", get(list_users))
        .add("/{user_id}", get(get_user))
        .add("/", post(create_user))
        .add("/{user_id}", put(update_user))
        .add("/{user_id}", delete(delete_user))
}

async fn list_users(auth: JWT, query: Query<ListUsersQuery>) -> Result<Response, AppError> {
    Ok(format::json(Vec::<UserResponse>::new()))
}

async fn get_user(auth: JWT) -> Result<Response, AppError> {
    Ok(format::json(UserResponse::from(user)))
}

async fn create_user(req: JsonValidate<CreateUserRequest>) -> Result<Response, AppError> {
    Err(BadRequest::EmailAlreadyExists.into())
}

async fn update_user(req: Json<UpdateUserRequest>) -> Result<Response, AppError> {
    Ok(format::json(UserResponse::from(user)))
}

async fn delete_user(auth: JWT) -> Result<Response, AppError> {
    Err(Forbidden.into())
}
"#,
    )?;

    std::fs::write(
        controllers.join("posts.rs"),
        r#"
pub fn routes() -> Router {
    Router::new()
        .route("/api/posts", get(list_posts))
        .route("/api/posts/{post_id}/comments/{comment_id}", get(get_comment))
        .nest("/api/admin", Router::new().route("/stats", get(admin_stats)))
}
"#,
    )?;

    std::fs::write(
        controllers.join("admin").join("legacy.rs"),
        r#"
pub fn routes() -> Router {
    Router::new()
        .route("/legacy/:legacy_id", get(get_legacy))
        .route("/legacy/:legacy_id", put(update_legacy))
}
"#,
    )?;

    // mod.rs and non-Rust files are ignored by the scanner.
    std::fs::write(controllers.join("mod.rs"), "pub mod users;")?;
    std::fs::write(controllers.join("notes.txt"), "not rust")?;
    Ok(())
}

fn base_config(dir: &Path) -> Config {
    Config {
        controllers_path: dir.join("controllers"),
        ..Config::default()
    }
}

#[test]
fn scans_all_fixture_styles() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("controllers"), "").expect("placeholder");
    std::fs::remove_file(dir.path().join("controllers")).expect("remove placeholder");
    write_fixtures(dir.path()).expect("fixtures");

    let routes = scan_routes(&base_config(dir.path())).expect("scan");
    let paths: Vec<String> = routes.iter().map(|route| route.path.clone()).collect();
    assert_eq!(
        paths,
        vec![
            // admin/legacy.rs is scanned first (sorted order).
            "/legacy/{legacy_id}",
            "/legacy/{legacy_id}",
            // posts.rs
            "/api/posts",
            "/api/posts/{post_id}/comments/{comment_id}",
            "/api/admin/stats",
            // users.rs
            "/api/users",
            "/api/users/{user_id}",
            "/api/users",
            "/api/users/{user_id}",
            "/api/users/{user_id}",
        ]
    );

    let create_user = routes
        .iter()
        .find(|route| route.name == "post_api_users")
        .expect("create_user route");
    assert_eq!(create_user.method, "POST");
    assert_eq!(
        create_user.handler_info.body_param.as_deref(),
        Some("CreateUserRequest")
    );
    assert!(
        create_user
            .handler_info
            .return_type
            .error_types
            .contains("BadRequest")
    );

    let list_users = routes
        .iter()
        .find(|route| route.name == "get_api_users")
        .expect("list_users route");
    assert!(list_users.handler_info.requires_auth);
    assert_eq!(
        list_users.handler_info.query_params.as_deref(),
        Some("ListUsersQuery")
    );
    assert_eq!(
        list_users.handler_info.return_type.found_type.as_deref(),
        Some("Array<UserResponse>")
    );
}

#[test]
fn generate_links_produces_the_expected_enum() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_fixtures(dir.path()).expect("fixtures");

    let mut config = base_config(dir.path());
    config.rust = RustConfig {
        output_file: Some(dir.path().join("generated/links.rs")),
        ..RustConfig::default()
    };
    let code = generate_links(&config).expect("generate");
    assert!(code.contains("pub enum Link {"));
    assert!(code.contains("GetApiUsers,"));
    assert!(code.contains("GetApiUsersUserId { user_id: String },"));
    assert!(
        code.contains(
            "GetApiPostsPostIdCommentsCommentId { post_id: String, comment_id: String },"
        )
    );
    assert!(code.contains("PostApiUsers,"));
    assert!(code.contains("DeleteApiUsersUserId { user_id: String },"));
    assert!(dir.path().join("generated/links.rs").exists());
}

#[test]
fn typescript_outputs_are_stable_and_typed() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_fixtures(dir.path()).expect("fixtures");

    let mut config = base_config(dir.path());
    config.typescript = TypeScriptConfig {
        generate_client: Some(true),
        output_path: Some(dir.path().join("web/api")),
        ..TypeScriptConfig::default()
    };
    let client = generate_ts_client(&config).expect("client");
    assert!(client.contains("export interface GetApiUsersUserIdParams {"));

    assert!(client
        .contains("postApiUsers: async (body: CreateUserRequest, config?: { signal?: AbortSignal }): Promise<any> => {"));
    assert!(client.contains(
        "getApiUsersUserId: async (params: GetApiUsersUserIdParams, config?: { signal?: AbortSignal }): Promise<UserResponse> => {"
    ));
    assert!(client.contains(
        "deleteApiUsersUserId: async (params: DeleteApiUsersUserIdParams, config?: { signal?: AbortSignal }): Promise<any> => {"
    ));

    assert!(client
        .contains("return apiClient.get<Array<UserResponse>, ApiError>(url, { requiresAuth: true, signal: config?.signal });"));
    assert!(client
        .contains("return apiClient.post<any, ApiError | BadRequest>(url, body, { requiresAuth: false, signal: config?.signal });"));
    assert!(!client.contains("TOKEN_KEY"));

    let hooks = generate_ts_hooks(&config).expect("hooks");
    assert!(hooks.contains("export function useGetApiUsersUserId(params: GetApiUsersUserIdParams, options?: Omit<UseQueryOptions<UserResponse, ApiError>, \"queryKey\">) {"));
    assert!(hooks.contains("export function usePostApiUsers(options?: UseMutationOptions<any, ApiError, CreateUserRequest, unknown>) {"));
}

#[test]
fn openapi_and_route_map_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_fixtures(dir.path()).expect("fixtures");
    let config = base_config(dir.path());

    let openapi = generate_openapi_json(&config).expect("openapi");
    let parsed: serde_json::Value = serde_json::from_str(&openapi).expect("valid json");
    assert_eq!(parsed["openapi"], "3.1.0");
    assert!(parsed["paths"]["/api/users/{user_id}"]["delete"].is_object());
    assert!(parsed["paths"]["/legacy/{legacy_id}"]["put"].is_object());

    let route_map = generate_route_map_json(&config).expect("route map");
    let parsed: serde_json::Value = serde_json::from_str(&route_map).expect("valid json");
    let entries = parsed.as_array().expect("array");
    assert_eq!(entries.len(), 10);
    assert!(entries[0]["handlerInfo"].is_object());
}

#[test]
fn custom_config_shapes_everything() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_fixtures(dir.path()).expect("fixtures");

    let toml = format!(
        r#"
        controllers_path = "{}"
        [naming]
        include_method_in_names = false
        collision_strategy = "append-method"
        variant_prefix = "Api"
        [rust]
        enum_name = "ApiLink"
        derive_serde = true
        "#,
        dir.path().join("controllers").display()
    );
    let config = Config::from_toml_str(&toml).expect("config");
    let code = generate_links(&config).expect("generate");
    assert!(code.contains("pub enum ApiLink"));
    assert!(code.contains("::serde::Serialize"));
    // Without the method in names the GET and POST variants collide and the
    // strategy prefixes the HTTP verb to the decorated duplicate.
    assert!(code.contains("ApiApiUsers,"));
    assert!(code.contains("PostApiApiUsers,"));
}

#[test]
fn empty_controllers_folder_is_reported_but_not_fatal() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("controllers")).expect("empty controllers dir");
    let config = base_config(dir.path());
    let routes = scan_routes(&config).expect("scan");
    assert_eq!(routes, Vec::new());
    let code = generate_links(&config).expect("generate");
    assert!(code.contains("pub enum Link {}"));
}

#[test]
fn duplicate_routes_are_deduplicated() {
    let dir = tempfile::tempdir().expect("tempdir");
    let controllers = dir.path().join("controllers");
    std::fs::create_dir_all(&controllers).expect("dir");
    std::fs::write(
        controllers.join("dup.rs"),
        "fn routes() -> Router {\n  Router::new()\n    .route(\"/things\", get(a))\n    .route(\"/things\", get(b))\n}\n",
    )
    .expect("fixture");
    let routes = scan_routes(&base_config(dir.path())).expect("scan");
    assert_eq!(routes.len(), 1);
}

#[test]
fn inconsistent_parameter_names_are_detected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let controllers = dir.path().join("controllers");
    std::fs::create_dir_all(&controllers).expect("dir");
    std::fs::write(
        controllers.join("shape.rs"),
        "fn routes() -> Router {\n  Router::new()\n    .route(\"/items/{id}\", get(a))\n    .route(\"/items/{item_id}\", get(b))\n}\n",
    )
    .expect("fixture");
    let routes = scan_routes(&base_config(dir.path())).expect("scan");
    assert_eq!(routes.len(), 2);
    // The warning is emitted on stderr; both routes survive.
    assert_eq!(routes[0].path, "/items/{id}");
    assert_eq!(routes[1].path, "/items/{item_id}");
}

#[test]
fn invalid_controller_files_surface_parse_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let controllers = dir.path().join("controllers");
    std::fs::create_dir_all(&controllers).expect("dir");
    std::fs::write(controllers.join("bad.rs"), "fn broken( {").expect("fixture");
    let error = scan_routes(&base_config(dir.path())).expect_err("must fail");
    assert!(
        error
            .to_string()
            .contains("failed to parse controller file")
    );
}
