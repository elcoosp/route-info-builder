use convert_case::{Case, Casing};

pub fn generate_route_name(path: &str, method: &str, config: &super::NamingConfig) -> String {
    let include_method = config.include_method_in_names.unwrap_or(true);

    let mut name_path = path.to_string();

    // Remove prefix from name but keep it in the actual route path
    if let Some(prefix) = &config.path_prefix_to_remove {
        let normalized_prefix = prefix.trim_matches('/');
        let normalized_path = name_path.trim_matches('/');

        if normalized_path.starts_with(normalized_prefix) {
            // Remove the prefix from the name but NOT from the actual path
            name_path = normalized_path[normalized_prefix.len()..].to_string();
            // Remove leading slash if present
            name_path = name_path.trim_start_matches('/').to_string();

            // If we removed everything, set to empty (will become "root")
            if name_path.is_empty() {
                name_path = "".to_string();
            }
        }
    }

    // Clean the path for name generation (this only affects the name, not the actual URL)
    let clean_path = clean_route_path_for_name(&name_path, config);

    let base_name = if clean_path.is_empty() || clean_path == "/" {
        "root".to_string()
    } else {
        clean_path
    };

    let name = if include_method {
        format!("{}_{}", method.to_lowercase(), base_name)
    } else {
        base_name
    };

    // Apply final transformations to ensure valid identifier
    crate::utils::case::sanitize_identifier(&name)
}

fn clean_route_path_for_name(path: &str, config: &super::NamingConfig) -> String {
    let mut result = path.trim_matches('/').replace("//", "/");

    // Replace parameter placeholders
    result = result.replace('{', "").replace('}', "");

    // Use custom word separators if specified
    if let Some(separators) = &config.word_separators {
        for sep in separators.chars() {
            result = result.replace(sep, "_");
        }
    } else {
        // Default separators
        result = result.replace(['-', '/', '.', ':'], "_");
    }

    // Remove duplicate underscores and trim
    while result.contains("__") {
        result = result.replace("__", "_");
    }

    result.trim_matches('_').to_string()
}
