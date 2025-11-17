use convert_case::{Case, Casing};

pub fn convert_to_case(input: &str, case: &str) -> String {
    match case.to_lowercase().as_str() {
        "camel" | "camelcase" => input.to_case(Case::Camel),
        "pascal" | "pascalcase" => input.to_case(Case::Pascal),
        "snake" | "snake_case" => input.to_case(Case::Snake),
        "kebab" | "kebab-case" => input.to_case(Case::Kebab),
        "title" | "title_case" => input.to_case(Case::Title),
        "lower" | "lowercase" => input.to_lowercase(),
        "upper" | "uppercase" => input.to_uppercase(),
        _ => input.to_string(), // Return as-is if unknown case
    }
}

pub fn sanitize_identifier(name: &str) -> String {
    let mut result = String::new();
    let mut chars = name.chars().peekable();

    // Ensure the identifier starts with a letter or underscore
    if let Some(&first) = chars.peek()
        && !first.is_alphabetic() && first != '_' {
            result.push('_');
        }

    for c in chars {
        if c.is_alphanumeric() || c == '_' {
            result.push(c);
        } else {
            result.push('_');
        }
    }

    result
}
