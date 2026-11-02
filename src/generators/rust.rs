//! Rust `Link` enum generator.
//!
//! The generated source is pretty-printed with `prettyplease` so it can be
//! written straight into a project without a formatter pass.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::fmt::Write as _;

use super::CodeGenerator;
use crate::configuration::naming::{VariantNamer, field_name, variant_base_name};
use crate::configuration::{Config, NamingConfig};
use crate::error::Result;
use crate::parser::RouteInfo;
use crate::utils::{build_warning, case, path};

/// Generates the typed link enum for all scanned routes.
pub struct LinksGenerator;

impl CodeGenerator for LinksGenerator {
    type Config = Config;
    type Output = String;

    fn generate(routes: &[RouteInfo], config: &Self::Config) -> Result<Self::Output> {
        let enum_ident = ident_from(&case::to_valid_ident(config.rust.enum_name()));
        let mut namer = VariantNamer::new(config.naming.collision_strategy());

        let mut variant_defs: Vec<TokenStream> = Vec::new();
        let mut path_arms: Vec<TokenStream> = Vec::new();
        let mut method_arms: Vec<TokenStream> = Vec::new();
        let mut name_arms: Vec<TokenStream> = Vec::new();

        for route in routes {
            let base = variant_base_name(&route.name, &config.naming);
            let Some(variant) = namer.assign(&base, &route.method) else {
                build_warning(&format!(
                    "route-info-builder: duplicate variant name `{base}` skipped for route {} {}",
                    route.method, route.path
                ));
                continue;
            };
            let decorated = ident_from(&case::to_valid_ident(&variant));
            let fields = path_fields(&route.path, &config.naming);

            let path_str = route.path.as_str();
            let method_str = route.method.as_str();
            let ts_name = case::convert_to_case(&variant, "camel");

            if fields.is_empty() {
                variant_defs.push(quote! { #decorated });
                path_arms.push(quote! { #enum_ident::#decorated => #path_str.to_string() });
                method_arms.push(quote! { #enum_ident::#decorated => #method_str });
            } else {
                let (template, format_args) = format_template(&route.path, &config.naming);
                let field_names: Vec<Ident> =
                    fields.iter().map(|field| ident_from(field)).collect();
                let declarations = field_names.iter().map(|field| quote! { #field: String });
                variant_defs.push(quote! { #decorated { #(#declarations),* } });
                path_arms.push(quote! {
                    #enum_ident::#decorated { #(#field_names),* } => format!(#template, #(#format_args),*)
                });
                method_arms.push(quote! { #enum_ident::#decorated { .. } => #method_str });
            }
            name_arms.push(quote! { #enum_ident::#decorated => #ts_name });
        }

        let derives = derive_attribute(config.rust.derive_serde());
        let generated = quote! {
            #[doc = "Auto-generated link enum for all application routes."]
            #derives
            pub enum #enum_ident {
                #(#variant_defs,)*
            }

            impl #enum_ident {
                /// Converts the link to a URL path string.
                pub fn to_path(&self) -> String {
                    match self {
                        #(#path_arms,)*
                    }
                }

                /// Returns the HTTP method of this route.
                pub fn method(&self) -> &'static str {
                    match self {
                        #(#method_arms,)*
                    }
                }

                /// Returns a stable camelCase name that matches the key used
                /// by the generated TypeScript client.
                pub fn name(&self) -> &'static str {
                    match self {
                        #(#name_arms,)*
                    }
                }
            }

            impl std::fmt::Display for #enum_ident {
                fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(formatter, "{}", self.to_path())
                }
            }
        };
        Ok(format_tokens(&generated))
    }
}

/// Builds a syntactically valid identifier.
fn ident_from(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

/// The derive list for the generated enum, with optional serde derives.
fn derive_attribute(with_serde: bool) -> TokenStream {
    if with_serde {
        quote! {
            #[derive(Debug, Clone, PartialEq, Eq, Hash, ::serde::Serialize, ::serde::Deserialize)]
        }
    } else {
        quote! { #[derive(Debug, Clone, PartialEq, Eq, Hash)] }
    }
}

/// Computes the sorted, deduplicated field names of a route path.
fn path_fields(route_path: &str, config: &NamingConfig) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    for segment in route_path.split('/') {
        if let Some(param) = path::param_name(segment)
            && !fields.iter().any(|field| field == param)
        {
            fields.push(field_name(param, config));
        }
    }
    fields
}

/// Builds the `format!` template and argument idents for a parameterized path.
fn format_template(route_path: &str, config: &NamingConfig) -> (String, Vec<Ident>) {
    let fields = path_fields(route_path, config);
    let mut template = String::new();
    for segment in route_path.split('/').filter(|segment| !segment.is_empty()) {
        template.push('/');
        match path::param_name(segment) {
            Some(param) => {
                let field = field_name(param, config);
                let index = fields
                    .iter()
                    .position(|candidate| *candidate == field)
                    .unwrap_or_default();
                let _unused = write!(template, "{{{index}}}");
            }
            None => template.push_str(segment),
        }
    }
    if template.is_empty() {
        template.push('/');
    }
    (
        template,
        fields.iter().map(|field| ident_from(field)).collect(),
    )
}

/// Pretty-prints generated tokens, falling back to the raw token string.
fn format_tokens(tokens: &TokenStream) -> String {
    match syn::parse2::<syn::File>(tokens.clone()) {
        Ok(file) => prettyplease::unparse(&file),
        Err(_) => tokens.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{format_template, format_tokens, path_fields};
    use crate::configuration::{Config, NamingConfig};
    use crate::error::Result;
    use crate::generators::CodeGenerator;
    use crate::parser::{HandlerInfo, RouteInfo};
    use proc_macro2::Ident;
    use quote::quote;

    fn route(name: &str, path: &str, method: &str) -> RouteInfo {
        RouteInfo {
            name: name.to_string(),
            path: path.to_string(),
            method: method.to_string(),
            handler: "handler".to_string(),
            handler_info: HandlerInfo::default(),
        }
    }

    #[test]
    fn empty_routes_produce_empty_enum() -> Result<()> {
        let code = super::LinksGenerator::generate(&[], &Config::default())?;
        assert!(code.contains("pub enum Link {}"));
        Ok(())
    }

    #[test]
    fn unit_and_parameterized_variants_are_generated() -> Result<()> {
        let routes = vec![
            route("get_users", "/api/users", "GET"),
            route("get_user", "/api/users/{user_id}", "GET"),
        ];
        let code = super::LinksGenerator::generate(&routes, &Config::default())?;
        assert!(code.contains("pub enum Link {"));
        assert!(code.contains("GetUsers,"));
        assert!(code.contains("GetUser { user_id: String },"));
        assert!(code.contains("Link::GetUsers => \"/api/users\".to_string()"));
        assert!(code.contains("Link::GetUser { user_id } => format!(\"/api/users/{0}\", user_id)"));
        assert!(code.contains("pub fn method(&self) -> &'static str"));
        assert!(code.contains("pub fn name(&self) -> &'static str"));
        assert!(code.contains("impl std::fmt::Display for Link"));
        Ok(())
    }

    #[test]
    fn repeated_parameters_use_numbered_placeholders() {
        let (template, args) = format_template("/x/{id}/y/{id}", &NamingConfig::default());
        assert_eq!(template, "/x/{0}/y/{0}");
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn root_path_template_is_root() {
        let (template, args) = format_template("/", &NamingConfig::default());
        assert_eq!(template, "/");
        assert_eq!(args, Vec::<Ident>::new());
    }

    #[test]
    fn path_fields_are_snake_case_and_unique() {
        let fields = path_fields("/users/{user_id}/posts/{user_id}", &NamingConfig::default());
        assert_eq!(fields, vec!["user_id".to_string()]);
    }

    #[test]
    fn custom_enum_name_and_serde_derives() -> Result<()> {
        let mut config = Config::default();
        config.rust.enum_name = Some("ApiLink".to_string());
        config.rust.derive_serde = Some(true);
        let routes = vec![route("get_users", "/users", "GET")];
        let code = super::LinksGenerator::generate(&routes, &config)?;
        assert!(code.contains("pub enum ApiLink"));
        assert!(code.contains("::serde::Serialize"));
        assert!(code.contains("::serde::Deserialize"));
        Ok(())
    }

    #[test]
    fn collision_strategies_shape_the_enum() -> Result<()> {
        let routes = vec![
            route("get_root", "/", "GET"),
            route("get_root", "/", "POST"),
            route("get_root", "/", "PUT"),
        ];
        let mut config = Config::default();
        config.naming.collision_strategy = Some(crate::configuration::CollisionStrategy::Number);
        let code = super::LinksGenerator::generate(&routes, &config)?;
        assert!(code.contains("GetRoot,"));
        assert!(code.contains("GetRoot2,"));
        assert!(code.contains("GetRoot3,"));

        config.naming.collision_strategy =
            Some(crate::configuration::CollisionStrategy::AppendMethod);
        let code = super::LinksGenerator::generate(&routes, &config)?;
        assert!(code.contains("GetRoot,"));
        assert!(code.contains("PostGetRoot,"));
        Ok(())
    }

    #[test]
    fn skip_strategy_warns_and_drops_duplicates() -> Result<()> {
        let routes = vec![
            route("get_root", "/", "GET"),
            route("get_root", "/", "POST"),
        ];
        let code = super::LinksGenerator::generate(&routes, &Config::default())?;
        // Only the first route survives; the duplicate is warned about.
        assert!(code.contains("GetRoot,"));
        assert!(!code.contains("GetRoot2"));
        Ok(())
    }

    #[test]
    fn format_tokens_falls_back_for_invalid_files() {
        let raw = format_tokens(&quote! { + });
        assert_eq!(raw, "+");
    }
}
