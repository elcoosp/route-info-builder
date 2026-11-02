//! `OpenAPI` 3.1 skeleton generator.
//!
//! The document is a *skeleton*: request/response schemas reference type
//! names collected from the handlers (`#/components/schemas/<Type>`), which
//! is enough to feed tooling and to be filled in by hand afterwards.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use super::CodeGenerator;
use crate::configuration::OpenApiConfig;
use crate::error::Result;
use crate::generators::typescript::method_display_name;
use crate::parser::RouteInfo;

/// Generates a deterministic `OpenAPI` 3.1 document from the scanned routes.
pub struct OpenApiGenerator;

impl CodeGenerator for OpenApiGenerator {
    type Config = OpenApiConfig;
    type Output = String;

    fn generate(routes: &[RouteInfo], config: &Self::Config) -> Result<Self::Output> {
        let mut paths: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
        let mut schemas: BTreeSet<String> = BTreeSet::new();

        for route in routes {
            let operation = operation_value(route, &mut schemas);
            paths
                .entry(route.path.clone())
                .or_default()
                .insert(route.method.to_lowercase(), operation);
        }

        for route in routes {
            if let Some(body) = &route.handler_info.body_param {
                schemas.insert(body.clone());
            }
            if let Some(query) = &route.handler_info.query_params {
                schemas.insert(query.clone());
            }
        }

        let schema_values: BTreeMap<String, Value> = schemas
            .iter()
            .map(|name| (name.clone(), json!({})))
            .collect();

        let document = json!({
            "openapi": "3.1.0",
            "info": {
                "title": config.title(),
                "version": config.version(),
            },
            "paths": paths,
            "components": {
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer",
                    }
                },
                "schemas": schema_values,
            },
        });
        Ok(serde_json::to_string_pretty(&document)?)
    }
}

/// Builds the operation object for a single route.
fn operation_value(route: &RouteInfo, schemas: &mut BTreeSet<String>) -> Value {
    let mut operation = serde_json::Map::new();
    operation.insert("operationId".to_string(), json!(method_display_name(route)));

    let mut parameters = Vec::new();
    for segment in route.path.split('/').filter(|segment| !segment.is_empty()) {
        if let Some(name) = crate::utils::path::param_name(segment) {
            parameters.push(json!({
                "name": name,
                "in": "path",
                "required": true,
                "schema": { "type": "string" },
            }));
        }
    }
    if let Some(query) = &route.handler_info.query_params {
        schemas.insert(query.clone());
        parameters.push(json!({
            "name": method_display_name(route),
            "in": "query",
            "required": false,
            "schema": { "$ref": format!("#/components/schemas/{query}") },
        }));
    }
    if !parameters.is_empty() {
        operation.insert("parameters".to_string(), json!(parameters));
    }

    if let Some(body) = &route.handler_info.body_param {
        schemas.insert(body.clone());
        operation.insert(
            "requestBody".to_string(),
            json!({
                "required": true,
                "content": {
                    "application/json": {
                        "schema": { "$ref": format!("#/components/schemas/{body}") },
                    }
                },
            }),
        );
    }

    let success_schema = match &route.handler_info.return_type.found_type {
        Some(found) if found != "any" => json!({ "$ref": format!("#/components/schemas/{found}") }),
        _ => json!({}),
    };
    if let Some(found) = &route.handler_info.return_type.found_type
        && found != "any"
    {
        schemas.insert(found.clone());
    }

    let mut error_schemas = vec![json!({ "$ref": "#/components/schemas/ApiError" })];
    for error_type in &route.handler_info.return_type.error_types {
        schemas.insert(error_type.clone());
        error_schemas.push(json!({ "$ref": format!("#/components/schemas/{error_type}") }));
    }
    let responses = json!({
        "200": {
            "description": "Success",
            "content": { "application/json": { "schema": success_schema } },
        },
        "4XX": {
            "description": "Error",
            "content": { "application/json": { "schema": { "anyOf": error_schemas } } },
        },
    });
    operation.insert("responses".to_string(), responses);

    if route.handler_info.requires_auth {
        operation.insert("security".to_string(), json!([{ "bearerAuth": [] }]));
    }

    Value::Object(operation)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::OpenApiGenerator;
    use crate::configuration::OpenApiConfig;
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
    fn generates_valid_skeleton_for_mixed_routes() {
        let mut user_route = route("/api/users/{user_id}", "GET", "get_user");
        user_route.handler_info = HandlerInfo {
            body_param: None,
            query_params: Some("ListUsersQuery".to_string()),
            requires_auth: true,
            return_type: ReturnType {
                found_type: Some("UserResponse".to_string()),
                is_importable: true,
                error_types: BTreeSet::from(["BadRequest".to_string()]),
                error_variants: BTreeSet::new(),
            },
        };
        let mut create_route = route("/api/users", "POST", "create_user");
        create_route.handler_info = HandlerInfo {
            body_param: Some("CreateUserRequest".to_string()),
            query_params: None,
            requires_auth: false,
            return_type: ReturnType::default(),
        };

        let openapi_config = OpenApiConfig {
            output_path: None,
            title: Some("Demo".to_string()),
            version: Some("9.9.9".to_string()),
        };
        let document = OpenApiGenerator::generate(&[user_route, create_route], &openapi_config)
            .expect("generate");
        let parsed: serde_json::Value = serde_json::from_str(&document).expect("valid json");

        assert_eq!(parsed["openapi"], "3.1.0");
        assert_eq!(parsed["info"]["title"], "Demo");
        assert_eq!(parsed["info"]["version"], "9.9.9");
        assert_eq!(
            parsed["paths"]["/api/users/{user_id}"]["get"]["operationId"],
            "getUser"
        );
        let parameters = &parsed["paths"]["/api/users/{user_id}"]["get"]["parameters"];
        assert_eq!(parameters.as_array().map(Vec::len), Some(2));
        assert_eq!(parameters[0]["name"], "user_id");
        assert_eq!(parameters[0]["in"], "path");
        assert_eq!(
            parameters[1]["schema"]["$ref"],
            "#/components/schemas/ListUsersQuery"
        );
        assert_eq!(
            parsed["paths"]["/api/users/{user_id}"]["get"]["security"][0]["bearerAuth"],
            json!([])
        );
        let error_schema = &parsed["paths"]["/api/users/{user_id}"]["get"]["responses"]["4XX"]["content"]
            ["application/json"]["schema"]["anyOf"];
        assert_eq!(error_schema.as_array().map(Vec::len), Some(2));
        assert_eq!(
            parsed["paths"]["/api/users"]["post"]["requestBody"]["content"]["application/json"]["schema"]
                ["$ref"],
            "#/components/schemas/CreateUserRequest"
        );
        assert_eq!(
            parsed["paths"]["/api/users"]["post"]["responses"]["200"]["content"]["application/json"]
                ["schema"],
            json!({})
        );
        let schemas = parsed["components"]["schemas"]
            .as_object()
            .expect("schemas");
        for expected in [
            "ListUsersQuery",
            "UserResponse",
            "BadRequest",
            "CreateUserRequest",
        ] {
            assert!(schemas.contains_key(expected), "missing schema {expected}");
        }
        assert_eq!(
            parsed["components"]["securitySchemes"]["bearerAuth"]["type"],
            "http"
        );
    }

    #[test]
    fn any_return_type_stays_schemaless() -> Result<()> {
        let route = route("/health", "GET", "get_health");
        let document = OpenApiGenerator::generate(&[route], &OpenApiConfig::default())?;
        let parsed: serde_json::Value = serde_json::from_str(&document).expect("valid json");
        assert_eq!(
            parsed["paths"]["/health"]["get"]["responses"]["200"]["content"]["application/json"]["schema"],
            json!({})
        );
        assert_eq!(parsed["info"]["title"], "API");
        assert!(parsed["paths"]["/health"]["get"].get("security").is_none());
        assert!(
            parsed["paths"]["/health"]["get"]
                .get("parameters")
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn output_is_deterministic() -> Result<()> {
        let routes = vec![route("/a", "GET", "get_a"), route("/b", "POST", "post_b")];
        let first = OpenApiGenerator::generate(&routes, &OpenApiConfig::default())?;
        let second = OpenApiGenerator::generate(&routes, &OpenApiConfig::default())?;
        assert_eq!(first, second);
        Ok(())
    }
}
