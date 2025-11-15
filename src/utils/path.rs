use std::collections::HashSet;

pub fn extract_parameters_from_path(path: &str) -> Vec<String> {
    let mut params = Vec::new();

    for segment in path.split('/') {
        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = segment[1..segment.len() - 1].to_string();
            params.push(param_name);
        }
    }

    // Remove duplicates while preserving order
    let mut seen = HashSet::new();
    params.retain(|param| seen.insert(param.clone()));

    params
}

pub fn build_full_path(prefix: &str, path: &str) -> String {
    if prefix.is_empty() {
        path.to_string()
    } else {
        let clean_prefix = prefix.trim_end_matches('/');
        let clean_path = path.trim_start_matches('/');
        format!("{}/{}", clean_prefix, clean_path)
    }
}
