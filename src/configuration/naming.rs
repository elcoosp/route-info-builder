//! Naming rules: route name generation and variant collision resolution.

use std::collections::BTreeSet;

use super::{CollisionStrategy, NamingConfig};
use crate::utils::case::{convert_to_case, convert_to_case_preserve_numbers, sanitize_identifier};

/// Generates the base name for a route from its normalized path and method.
///
/// The configured `path_prefix_to_remove` only affects the name — the URL
/// path keeps its prefix. Prefix matching is segment-aware: removing `/user`
/// does not touch a `/users` route.
#[must_use]
pub(crate) fn generate_route_name(path: &str, method: &str, config: &NamingConfig) -> String {
    let mut name_path = path;
    if let Some(prefix) = &config.path_prefix_to_remove {
        name_path = strip_name_prefix(name_path, prefix);
    }

    let base_name = clean_route_path_for_name(name_path, config);
    let base_name = if base_name.is_empty() {
        "root"
    } else {
        base_name.as_str()
    };

    let name = if config.include_method() {
        format!("{}_{}", method.to_lowercase(), base_name)
    } else {
        base_name.to_string()
    };

    sanitize_identifier(&name)
}

/// Removes `prefix` from the start of `path` only when it matches whole
/// path segments.
fn strip_name_prefix<'a>(path: &'a str, prefix: &str) -> &'a str {
    let prefix = prefix.trim_matches('/');
    let whole = path.trim_start_matches('/');
    if prefix.is_empty() {
        return whole;
    }
    match whole.strip_prefix(prefix) {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => rest.trim_start_matches('/'),
        _ => whole,
    }
}

/// Cleans a path for name generation: drops parameter braces, replaces
/// separators with underscores and collapses runs of separators.
///
/// The `/` segment divider always acts as a separator; `word_separators`
/// adds further characters on top of the default set (`-`, `.`, `:`).
fn clean_route_path_for_name(path: &str, config: &NamingConfig) -> String {
    let without_params: String = path.chars().filter(|ch| *ch != '{' && *ch != '}').collect();
    let normalized: String = without_params
        .chars()
        .map(|ch| {
            if matches!(ch, '-' | '/' | '.' | ':')
                || config
                    .word_separators
                    .as_ref()
                    .is_some_and(|custom| custom.contains(ch))
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    normalized
        .split('_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Assigns unique variant names according to the configured
/// [`CollisionStrategy`].
pub(crate) struct VariantNamer {
    used: BTreeSet<String>,
    strategy: CollisionStrategy,
}

impl VariantNamer {
    /// Creates a namer for the given strategy.
    pub(crate) fn new(strategy: CollisionStrategy) -> Self {
        Self {
            used: BTreeSet::new(),
            strategy,
        }
    }

    /// Reserves a variant name derived from `base`.
    ///
    /// Returns [`None`] when the strategy is [`CollisionStrategy::Skip`] and
    /// `base` was already taken; the caller is expected to warn about the
    /// skipped route.
    pub(crate) fn assign(&mut self, base: &str, method: &str) -> Option<String> {
        if !self.used.contains(base) {
            self.used.insert(base.to_string());
            return Some(base.to_string());
        }
        match self.strategy {
            CollisionStrategy::Skip => None,
            CollisionStrategy::AppendMethod => {
                let candidate = format!("{}{base}", method_pascal(method));
                if self.used.contains(&candidate) {
                    return Some(self.numbered(base));
                }
                self.used.insert(candidate.clone());
                Some(candidate)
            }
            CollisionStrategy::Number => Some(self.numbered(base)),
        }
    }

    /// Appends increasing counters until an unused name appears.
    fn numbered(&mut self, base: &str) -> String {
        let mut counter = 2;
        loop {
            let candidate = format!("{base}{counter}");
            if !self.used.contains(&candidate) {
                self.used.insert(candidate.clone());
                return candidate;
            }
            counter += 1;
        }
    }
}

/// Pascal-cases an HTTP method for use in variant names (`GET` becomes `Get`).
fn method_pascal(method: &str) -> String {
    convert_to_case(method, "pascal")
}

/// Applies the naming config to an already generated base name, producing the
/// final variant identifier body (case conversion plus prefix/suffix).
#[must_use]
pub(crate) fn variant_base_name(name: &str, config: &NamingConfig) -> String {
    let case = config.variant_case();
    let converted = if config.preserve_numbers == Some(false) {
        convert_to_case_preserve_numbers(name, case)
    } else {
        convert_to_case(name, case)
    };
    let mut result = converted;
    if let Some(prefix) = &config.variant_prefix {
        result = format!("{prefix}{result}");
    }
    if let Some(suffix) = &config.variant_suffix {
        result = format!("{result}{suffix}");
    }
    sanitize_identifier(&result)
}

/// Converts a route parameter into a field identifier body.
#[must_use]
pub(crate) fn field_name(param: &str, config: &NamingConfig) -> String {
    let case = config.field_case();
    let converted = if config.preserve_numbers == Some(false) {
        convert_to_case_preserve_numbers(param, case)
    } else {
        convert_to_case(param, case)
    };
    sanitize_identifier(&converted)
}

#[cfg(test)]
mod tests {
    use super::{
        VariantNamer, field_name, generate_route_name, strip_name_prefix, variant_base_name,
    };
    use crate::configuration::{CollisionStrategy, NamingConfig};

    fn config() -> NamingConfig {
        NamingConfig::default()
    }

    #[test]
    fn includes_method_by_default() {
        let name = generate_route_name("/api/users", "GET", &config());
        assert_eq!(name, "get_api_users");
    }

    #[test]
    fn can_exclude_method() {
        let mut naming = config();
        naming.include_method_in_names = Some(false);
        let name = generate_route_name("/api/users", "GET", &naming);
        assert_eq!(name, "api_users");
    }

    #[test]
    fn root_path_becomes_root() {
        let name = generate_route_name("/", "GET", &config());
        assert_eq!(name, "get_root");
    }

    #[test]
    fn strips_prefix_segment_aware() {
        let mut naming = config();
        naming.path_prefix_to_remove = Some("/api".to_string());
        assert_eq!(
            generate_route_name("/api/users", "GET", &naming),
            "get_users"
        );
        // The near-miss prefix must NOT be stripped.
        assert_eq!(generate_route_name("/users", "GET", &naming), "get_users");
    }

    #[test]
    fn strips_prefix_without_leading_slash() {
        let mut naming = config();
        naming.path_prefix_to_remove = Some("api".to_string());
        assert_eq!(generate_route_name("/api", "GET", &naming), "get_root");
    }

    #[test]
    fn params_are_removed_from_names() {
        let name = generate_route_name("/api/users/{user_id}/posts", "GET", &config());
        assert_eq!(name, "get_api_users_user_id_posts");
    }

    #[test]
    fn custom_word_separators_are_applied() {
        let mut naming = config();
        naming.word_separators = Some("-.".to_string());
        let name = generate_route_name("/a-b.c/d", "GET", &naming);
        assert_eq!(name, "get_a_b_c_d");
    }

    #[test]
    fn variant_base_name_applies_case_and_decorations() {
        let mut naming = config();
        naming.variant_prefix = Some("Api".to_string());
        naming.variant_suffix = Some("Route".to_string());
        assert_eq!(variant_base_name("get_users", &naming), "ApiGetUsersRoute");
    }

    #[test]
    fn variant_base_name_preserves_numbers_when_configured() {
        let mut naming = config();
        naming.preserve_numbers = Some(false);
        assert_eq!(variant_base_name("get_v1_users", &naming), "GetV1Users");
    }

    #[test]
    fn field_name_uses_field_case() {
        let mut naming = config();
        naming.field_case = Some("camel".to_string());
        assert_eq!(field_name("user_id", &naming), "userId");
        assert_eq!(field_name("user-id", &naming), "userId");
    }

    #[test]
    fn field_name_can_preserve_numbers() {
        let mut naming = config();
        naming.field_case = Some("snake".to_string());
        naming.preserve_numbers = Some(false);
        assert_eq!(field_name("route_v2_id", &naming), "route_v2_id");
        assert_eq!(field_name("route_v2_id", &config()), "route_v_2_id");
    }

    #[test]
    fn strip_name_prefix_edge_cases() {
        assert_eq!(strip_name_prefix("/api/users", "/api"), "users");
        assert_eq!(strip_name_prefix("/api", "/api"), "");
        assert_eq!(strip_name_prefix("/users", ""), "users");
        assert_eq!(strip_name_prefix("/users", "/nope"), "users");
    }

    #[test]
    fn namer_skip_strategy_returns_none_on_collision() {
        let mut namer = VariantNamer::new(CollisionStrategy::Skip);
        assert_eq!(namer.assign("get_users", "GET"), Some("get_users".into()));
        assert_eq!(namer.assign("get_users", "POST"), None);
        assert_eq!(namer.assign("get_users", "PUT"), None);
        assert_eq!(
            namer.assign("post_users", "POST"),
            Some("post_users".into())
        );
    }

    #[test]
    fn namer_append_method_prefixes_http_verb() {
        let mut namer = VariantNamer::new(CollisionStrategy::AppendMethod);
        assert_eq!(namer.assign("users", "GET"), Some("users".into()));
        assert_eq!(namer.assign("users", "POST"), Some("Postusers".into()));
        assert_eq!(namer.assign("users", "PUT"), Some("Putusers".into()));
    }

    #[test]
    fn namer_number_strategy_appends_counters() {
        let mut namer = VariantNamer::new(CollisionStrategy::Number);
        assert_eq!(namer.assign("users", "GET"), Some("users".into()));
        assert_eq!(namer.assign("users", "POST"), Some("users2".into()));
        assert_eq!(namer.assign("users", "PATCH"), Some("users3".into()));
    }

    #[test]
    fn namer_append_method_falls_back_to_numbering() {
        let mut namer = VariantNamer::new(CollisionStrategy::AppendMethod);
        assert_eq!(namer.assign("users", "GET"), Some("users".into()));
        assert_eq!(namer.assign("users", "GET"), Some("Getusers".into()));
        assert_eq!(namer.assign("users", "POST"), Some("Postusers".into()));
        // The method-prefixed candidate itself collides: numbering kicks in.
        assert_eq!(namer.assign("users", "POST"), Some("users2".into()));
    }
}
