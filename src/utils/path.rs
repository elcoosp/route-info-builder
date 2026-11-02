//! Path helpers: parameter extraction, normalization and joining.
//!
//! Both the modern Axum 0.8 syntax (`{param}`, `{*wildcard}`) and the legacy
//! syntax (`:param`, `*wildcard`) are understood; parameters are always
//! reported with their bare name.

/// Returns the bare parameter name when `segment` is a path parameter.
///
/// Recognized forms: `{name}`, `{*name}`, `:name` and `*name`. Anything else
/// (including empty parameter names such as `{}`) is treated as a literal
/// segment and yields [`None`].
#[must_use]
pub(crate) fn param_name(segment: &str) -> Option<&str> {
    let inner = if segment.starts_with('{') && segment.ends_with('}') && segment.len() >= 2 {
        &segment[1..segment.len() - 1]
    } else {
        segment
            .strip_prefix(':')
            .or_else(|| segment.strip_prefix('*'))?
    };
    let name = inner.strip_prefix('*').unwrap_or(inner);
    if name.is_empty() { None } else { Some(name) }
}

/// Extracts the parameter names from a route path, in order of appearance,
/// without duplicates.
#[must_use]
pub(crate) fn extract_parameters_from_path(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    for segment in path.split('/') {
        if let Some(name) = param_name(segment)
            && !params.iter().any(|existing| existing == name)
        {
            params.push(name.to_string());
        }
    }
    params
}

/// Returns the canonical Axum 0.8 form of a path segment.
///
/// Legacy parameter syntax is rewritten: `:name` becomes `{name}` and
/// `*name`/`{*name}` become `{*name}`. Literal segments are unchanged.
#[must_use]
pub(crate) fn canonical_segment(segment: &str) -> String {
    if segment.starts_with('{') && segment.ends_with('}') && segment.len() >= 2 {
        let inner = &segment[1..segment.len() - 1];
        if let Some(name) = inner.strip_prefix('*')
            && !name.is_empty()
        {
            return format!("{{*{name}}}");
        }
        if !inner.is_empty() {
            return segment.to_string();
        }
        return segment.to_string();
    }
    if let Some(name) = segment.strip_prefix(':')
        && !name.is_empty()
    {
        return format!("{{{name}}}");
    }
    if let Some(name) = segment.strip_prefix('*')
        && !name.is_empty()
    {
        return format!("{{*{name}}}");
    }
    segment.to_string()
}

/// Normalizes a route path: leading slash, no trailing slash, collapsed
/// duplicate slashes and canonical `{param}` placeholders. The root path
/// stays `/`.
#[must_use]
pub(crate) fn normalize_path(path: &str) -> String {
    let segments: Vec<String> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(canonical_segment)
        .collect();
    if segments.is_empty() {
        return "/".to_string();
    }
    format!("/{}", segments.join("/"))
}

/// Joins a prefix and a path into one normalized route path.
#[must_use]
pub(crate) fn build_full_path(prefix: &str, path: &str) -> String {
    let prefix = prefix.trim_matches('/');
    let path = path.trim_matches('/');
    if prefix.is_empty() {
        normalize_path(path)
    } else if path.is_empty() {
        normalize_path(prefix)
    } else {
        normalize_path(&format!("{prefix}/{path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_full_path, canonical_segment, extract_parameters_from_path, normalize_path,
        param_name,
    };

    #[test]
    fn canonical_segment_rewrites_legacy_syntax() {
        assert_eq!(canonical_segment("{user_id}"), "{user_id}");
        assert_eq!(canonical_segment("{*rest}"), "{*rest}");
        assert_eq!(canonical_segment(":user_id"), "{user_id}");
        assert_eq!(canonical_segment("*rest"), "{*rest}");
        assert_eq!(canonical_segment("users"), "users");
        assert_eq!(canonical_segment(":"), ":");
        assert_eq!(canonical_segment("*"), "*");
        assert_eq!(canonical_segment("{}"), "{}");
    }

    #[test]
    fn param_name_modern_syntax() {
        assert_eq!(param_name("{user_id}"), Some("user_id"));
        assert_eq!(param_name("{*rest}"), Some("rest"));
    }

    #[test]
    fn param_name_legacy_syntax() {
        assert_eq!(param_name(":user_id"), Some("user_id"));
        assert_eq!(param_name("*rest"), Some("rest"));
    }

    #[test]
    fn param_name_rejects_literals_and_degenerate_params() {
        assert_eq!(param_name("users"), None);
        assert_eq!(param_name("{}"), None);
        assert_eq!(param_name("{*}"), None);
        assert_eq!(param_name(":"), None);
        assert_eq!(param_name("*"), None);
        assert_eq!(param_name(""), None);
        assert_eq!(param_name("{unterminated"), None);
    }

    #[test]
    fn extract_parameters_handles_mixed_syntax_and_dedup() {
        let params = extract_parameters_from_path("/api/{user_id}/posts/:post_id/{user_id}");
        assert_eq!(params, vec!["user_id".to_string(), "post_id".to_string()]);
    }

    #[test]
    fn extract_parameters_empty_path() {
        assert_eq!(extract_parameters_from_path("/users"), Vec::<String>::new());
        assert_eq!(extract_parameters_from_path("/"), Vec::<String>::new());
    }

    #[test]
    fn normalize_path_collapses_slashes() {
        assert_eq!(normalize_path("/api//users/"), "/api/users");
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path("api/users"), "/api/users");
    }

    #[test]
    fn normalize_path_canonicalizes_legacy_params() {
        assert_eq!(normalize_path("/users/:user_id"), "/users/{user_id}");
        assert_eq!(normalize_path("/files/*rest"), "/files/{*rest}");
        assert_eq!(normalize_path("/a//b/:id/*rest"), "/a/b/{id}/{*rest}");
    }

    #[test]
    fn build_full_path_joins_prefix() {
        assert_eq!(build_full_path("/api", "/users"), "/api/users");
        assert_eq!(build_full_path("/api/", "users"), "/api/users");
        assert_eq!(build_full_path("", "/users"), "/users");
        assert_eq!(build_full_path("/api", ""), "/api");
        assert_eq!(build_full_path("", ""), "/");
    }
}
