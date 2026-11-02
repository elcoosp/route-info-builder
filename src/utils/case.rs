//! Identifier construction: case conversion, sanitizing and keyword escaping.

use convert_case::{Boundary, Case, Casing};

/// Parses a case name in any of the accepted spellings.
///
/// Accepts `camel`/`camelCase`, `pascal`/`PascalCase`, `snake`/`snake_case`,
/// `kebab`/`kebab-case`, `title`/`Title Case`, `lower`/`lowercase` and
/// `upper`/`uppercase`. Unknown names return [`None`] so that callers can fall
/// back to their default.
#[must_use]
pub(crate) fn parse_case(case: &str) -> Option<Case<'static>> {
    let normalized = case.to_lowercase().replace(['-', '_', ' '], "");
    match normalized.as_str() {
        "camel" | "camelcase" => Some(Case::Camel),
        "pascal" | "pascalcase" => Some(Case::Pascal),
        "snake" | "snakecase" => Some(Case::Snake),
        "kebab" | "kebabcase" => Some(Case::Kebab),
        "title" | "titlecase" => Some(Case::Title),
        "lower" | "lowercase" => Some(Case::Flat),
        "upper" | "uppercase" => Some(Case::UpperFlat),
        _ => None,
    }
}

/// Converts `input` into the requested case, returning it unchanged when the
/// case name is unknown.
#[must_use]
pub(crate) fn convert_to_case(input: &str, case: &str) -> String {
    match parse_case(case) {
        Some(parsed) => input.to_case(parsed),
        None => input.to_string(),
    }
}

/// Converts `input` while gluing digits to the surrounding word.
///
/// Used for the `preserve_numbers = false` naming option: by default
/// `convert_case` treats a digit as a word boundary (`v1_user` becomes
/// `v_1_user` in snake case); with the digit boundaries removed the digits
/// stay attached (`v1_user` stays `v1_user`).
#[must_use]
pub(crate) fn convert_to_case_preserve_numbers(input: &str, case: &str) -> String {
    match parse_case(case) {
        Some(parsed) => input
            .remove_boundaries(&[
                Boundary::LowerDigit,
                Boundary::UpperDigit,
                Boundary::DigitLower,
                Boundary::DigitUpper,
            ])
            .to_case(parsed),
        None => input.to_string(),
    }
}

/// Rust strict and reserved keywords that cannot be used as plain identifiers.
const KEYWORDS: &[&str] = &[
    "as", "async", "await", "become", "box", "break", "const", "continue", "crate", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Keywords that cannot even be written as raw identifiers.
const NON_RAW_KEYWORDS: &[&str] = &["self", "Self", "super", "crate"];

/// Returns `true` when `word` is a Rust keyword.
#[must_use]
pub(crate) fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}

/// Sanitizes an arbitrary string into a valid Rust identifier body.
///
/// Runs of invalid characters collapse into a single `_`, a leading digit is
/// prefixed with `_`, and empty input becomes `_`.
///
/// Only ASCII digits and alphabetic characters are kept verbatim: some
/// `char::is_alphanumeric` code points (for example superscript digits) are
/// not valid in Rust identifiers, so they are treated as separators.
#[must_use]
pub(crate) fn sanitize_identifier(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 1);
    let mut previous_was_separator = false;
    for ch in name.chars() {
        let valid = ch == '_' || ch.is_ascii_digit() || ch.is_alphabetic();
        if valid {
            if result.is_empty() && ch.is_ascii_digit() {
                result.push('_');
            }
            result.push(ch);
            previous_was_separator = false;
        } else if !previous_was_separator {
            if !result.is_empty() {
                result.push('_');
            }
            previous_was_separator = true;
        }
    }
    if result.is_empty() {
        result.push('_');
    }
    result
}

/// Builds a syntactically usable Rust identifier string.
///
/// * the name is sanitized first (see [`sanitize_identifier`]),
/// * keywords are turned into raw identifiers (`type` becomes `r#type`),
/// * keywords that cannot be raw (`self`, `Self`, `super`, `crate`) get a
///   trailing underscore instead.
#[must_use]
pub(crate) fn to_valid_ident(name: &str) -> String {
    let sanitized = sanitize_identifier(name);
    if !is_keyword(&sanitized) {
        return sanitized;
    }
    if NON_RAW_KEYWORDS.contains(&sanitized.as_str()) {
        format!("{sanitized}_")
    } else {
        format!("r#{sanitized}")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        convert_to_case, convert_to_case_preserve_numbers, is_keyword, parse_case,
        sanitize_identifier, to_valid_ident,
    };

    #[test]
    fn parse_case_accepts_common_spellings() {
        use convert_case::Case;
        assert_eq!(parse_case("camel"), Some(Case::Camel));
        assert_eq!(parse_case("camelCase"), Some(Case::Camel));
        assert_eq!(parse_case("PascalCase"), Some(Case::Pascal));
        assert_eq!(parse_case("snake_case"), Some(Case::Snake));
        assert_eq!(parse_case("kebab-case"), Some(Case::Kebab));
        assert_eq!(parse_case("Title Case"), Some(Case::Title));
        assert_eq!(parse_case("lowercase"), Some(Case::Flat));
        assert_eq!(parse_case("UPPER"), Some(Case::UpperFlat));
        assert_eq!(parse_case("nonsense"), None);
    }

    #[test]
    fn convert_to_case_maps_known_cases() {
        assert_eq!(convert_to_case("get_users", "pascal"), "GetUsers");
        assert_eq!(convert_to_case("GetUsers", "camel"), "getUsers");
        assert_eq!(convert_to_case("GetUsers", "snake"), "get_users");
        assert_eq!(convert_to_case("GetUsers", "kebab"), "get-users");
        assert_eq!(convert_to_case("get users", "title"), "Get Users");
        assert_eq!(convert_to_case("GetUsers", "lower"), "getusers");
        assert_eq!(convert_to_case("GetUsers", "upper"), "GETUSERS");
    }

    #[test]
    fn convert_to_case_unknown_case_is_identity() {
        assert_eq!(convert_to_case("GetUsers", "gibberish"), "GetUsers");
    }

    #[test]
    fn preserve_numbers_keeps_digits_glued() {
        assert_eq!(
            convert_to_case_preserve_numbers("v1_user_list", "snake"),
            "v1_user_list"
        );
        assert_eq!(convert_to_case("v1_user_list", "snake"), "v_1_user_list");
        assert_eq!(
            convert_to_case_preserve_numbers("v1_user_list", "pascal"),
            "V1UserList"
        );
        assert_eq!(convert_to_case_preserve_numbers("abc", "nonsense"), "abc");
    }

    #[test]
    fn is_keyword_detects_strict_and_reserved_words() {
        assert!(is_keyword("type"));
        assert!(is_keyword("match"));
        assert!(is_keyword("gen"));
        assert!(is_keyword("Self"));
        assert!(!is_keyword("Match"));
        assert!(!is_keyword("user_id"));
    }

    #[test]
    fn sanitize_identifier_keeps_valid_chars() {
        assert_eq!(sanitize_identifier("get_users"), "get_users");
        assert_eq!(sanitize_identifier("user-name"), "user_name");
        assert_eq!(sanitize_identifier("a--b"), "a_b");
        assert_eq!(sanitize_identifier("2fast"), "_2fast");
        assert_eq!(sanitize_identifier(""), "_");
        assert_eq!(sanitize_identifier("___"), "___");
        assert_eq!(sanitize_identifier("///"), "_");
    }

    #[test]
    fn sanitize_identifier_handles_unicode() {
        assert_eq!(sanitize_identifier("café"), "café");
    }

    #[test]
    fn to_valid_ident_escapes_keywords() {
        assert_eq!(to_valid_ident("type"), "r#type");
        assert_eq!(to_valid_ident("match"), "r#match");
        assert_eq!(to_valid_ident("gen"), "r#gen");
        assert_eq!(to_valid_ident("self"), "self_");
        assert_eq!(to_valid_ident("Self"), "Self_");
        assert_eq!(to_valid_ident("crate"), "crate_");
        assert_eq!(to_valid_ident("user_id"), "user_id");
        assert_eq!(to_valid_ident("2fast"), "_2fast");
    }
}
