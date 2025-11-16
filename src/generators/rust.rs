use super::CodeGenerator;
use crate::{
    RouteInfo,
    config::{Config, NamingConfig},
};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;

pub struct RustLinksGenerator;

impl CodeGenerator for RustLinksGenerator {
    type Config = Config;
    type Output = String;

    fn generate(
        routes: &[RouteInfo],
        config: &Self::Config,
    ) -> Result<Self::Output, Box<dyn std::error::Error>> {
        let mut variants = Vec::new();
        let mut match_arms = Vec::new();
        let mut method_arms = Vec::new();

        // Use a HashMap to track unique variant names and avoid duplicates
        let mut unique_variants: HashMap<String, &RouteInfo> = HashMap::new();

        for route in routes {
            let variant_name = create_variant_name(&route.name, &config.naming);
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
            let path_params = crate::utils::path::extract_parameters_from_path(&route.path);

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
                        let field_name = create_field_name(param, &config.naming);
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

        Ok(generated.to_string())
    }
}

fn create_variant_name(name: &str, config: &NamingConfig) -> proc_macro2::Ident {
    let case = config.variant_case.as_deref().unwrap_or("pascal");
    let mut result = crate::utils::case::convert_to_case(name, case);

    // Apply prefix and suffix
    if let Some(prefix) = &config.variant_prefix {
        result = format!("{}{}", prefix, result);
    }

    if let Some(suffix) = &config.variant_suffix {
        result = format!("{}{}", result, suffix);
    }

    // Ensure it's a valid identifier
    let sanitized = crate::utils::case::sanitize_identifier(&result);
    syn::Ident::new(&sanitized, proc_macro2::Span::call_site())
}

fn create_field_name(name: &str, config: &NamingConfig) -> String {
    let case = config.field_case.as_deref().unwrap_or("snake");
    let result = crate::utils::case::convert_to_case(name, case);
    crate::utils::case::sanitize_identifier(&result)
}

fn generate_path_build_code(path_template: &str, fields: &[proc_macro2::Ident]) -> TokenStream {
    // Parse the path template and build a sequence of push operations
    let segments: Vec<&str> = path_template.split('/').filter(|s| !s.is_empty()).collect();
    let mut push_operations = Vec::new();

    for (i, segment) in segments.iter().enumerate() {
        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = &segment[1..segment.len() - 1];
            let field_ident = syn::Ident::new(
                &crate::utils::case::convert_to_case(param_name, "snake"),
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
