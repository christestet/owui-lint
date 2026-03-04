pub fn strip_inline_comment(line: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_single || in_double => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double => return &line[..idx],
            _ => {}
        }
    }
    line
}

pub fn count_indent(line: &str) -> usize {
    line.chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .map(|ch| if ch == '\t' { 4 } else { 1 })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::strip_inline_comment;

    #[test]
    fn strip_inline_comment_keeps_hash_inside_double_quoted_string() {
        let line = r#"message = "value # not a comment"  # actual comment"#;
        assert_eq!(
            strip_inline_comment(line).trim_end(),
            r#"message = "value # not a comment""#
        );
    }

    #[test]
    fn strip_inline_comment_handles_escaped_quote_before_hash() {
        let line = r#"message = "escaped quote \" # still string"  # comment"#;
        assert_eq!(
            strip_inline_comment(line).trim_end(),
            r#"message = "escaped quote \" # still string""#
        );
    }
}
