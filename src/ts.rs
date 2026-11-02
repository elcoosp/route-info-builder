//! Small text helpers used while emitting TypeScript sources.

/// Indents every non-empty line of `text` by `width` spaces.
///
/// Empty lines are left untouched so that blank separators stay blank.
#[must_use]
pub(crate) fn indent(text: &str, width: usize) -> String {
    let pad = " ".repeat(width);
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{pad}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::indent;

    #[test]
    fn indents_each_non_empty_line() {
        let text = "a\n\nb\nc";
        assert_eq!(indent(text, 2), "  a\n\n  b\n  c");
    }

    #[test]
    fn indents_with_large_width() {
        assert_eq!(indent("x", 4), "    x");
    }

    #[test]
    fn handles_empty_input() {
        assert_eq!(indent("", 2), "");
    }
}
