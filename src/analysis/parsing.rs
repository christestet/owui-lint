use crate::util::strip_inline_comment;

#[derive(Debug)]
pub(super) struct FunctionDef {
    pub(super) name: String,
    pub(super) args: Vec<String>,
    pub(super) is_async: bool,
    pub(super) returns_annotation: bool,
}

pub(super) fn is_function_start(trimmed: &str) -> bool {
    trimmed.starts_with("def ") || trimmed.starts_with("async def ")
}

pub(super) fn collect_function_signature(lines: &[&str], start_idx: usize) -> (String, usize) {
    let mut signature = strip_inline_comment(lines[start_idx].trim())
        .trim()
        .to_string();
    let mut consumed = 1;

    while !function_signature_complete(&signature) && start_idx + consumed < lines.len() {
        let next = strip_inline_comment(lines[start_idx + consumed].trim()).trim();
        if !next.is_empty() {
            if !signature.is_empty() {
                signature.push(' ');
            }
            signature.push_str(next);
        }
        consumed += 1;
    }

    (signature, consumed)
}

fn function_signature_complete(signature: &str) -> bool {
    let trimmed = signature.trim_end();
    if !trimmed.ends_with(':') {
        return false;
    }

    paren_balance(trimmed) == 0
}

pub(super) fn paren_balance(value: &str) -> i32 {
    let mut balance = 0_i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        if in_double {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }

        match ch {
            '\'' => in_single = true,
            '"' => in_double = true,
            '(' => balance += 1,
            ')' => balance -= 1,
            _ => {}
        }
    }

    balance
}

pub(super) fn parse_function_definition(trimmed: &str) -> Option<FunctionDef> {
    let (payload, is_async) = if let Some(payload) = trimmed.strip_prefix("async def ") {
        (payload, true)
    } else if let Some(payload) = trimmed.strip_prefix("def ") {
        (payload, false)
    } else {
        return None;
    };

    let open_paren = payload.find('(')?;
    let name = payload[..open_paren].trim();
    if !is_valid_identifier(name) {
        return None;
    }

    let closing_paren = payload.rfind(')')?;
    if closing_paren <= open_paren {
        return None;
    }

    let args_raw = &payload[open_paren + 1..closing_paren];
    let args = parse_args(args_raw);

    let tail = payload[closing_paren + 1..].trim();
    let returns_annotation = tail.starts_with("->");

    Some(FunctionDef {
        name: name.to_string(),
        args,
        is_async,
        returns_annotation,
    })
}

fn parse_args(raw: &str) -> Vec<String> {
    let mut args = Vec::new();
    for part in raw.split(',') {
        let token = part.trim();
        if token.is_empty() {
            continue;
        }
        let without_default = token.split('=').next().unwrap_or(token).trim();
        let without_type = without_default
            .split(':')
            .next()
            .unwrap_or(without_default)
            .trim();
        let normalized = without_type.trim_start_matches('*').trim();
        if is_valid_identifier(normalized) {
            args.push(normalized.to_string());
        }
    }

    args
}

pub(super) fn parse_class_definition(trimmed: &str) -> Option<(String, Vec<String>)> {
    let payload = trimmed.strip_prefix("class ")?;
    let class_name_end = payload
        .chars()
        .position(|ch| ch == '(' || ch == ':' || ch.is_whitespace())
        .unwrap_or(payload.len());
    let class_name = payload[..class_name_end].trim();
    if !is_valid_identifier(class_name) {
        return None;
    }

    let bases = if let (Some(open), Some(close)) = (payload.find('('), payload.rfind(')')) {
        if close > open {
            payload[open + 1..close]
                .split(',')
                .map(str::trim)
                .filter(|base| !base.is_empty())
                .map(|base| base.to_string())
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Some((class_name.to_string(), bases))
}

pub(super) fn parse_import(trimmed: &str) -> Option<String> {
    let payload = trimmed.strip_prefix("import ")?;
    let first = payload.split(',').next()?.trim();
    if first.is_empty() {
        return None;
    }
    Some(first.split_whitespace().next().unwrap_or(first).to_string())
}

pub(super) fn parse_import_from(trimmed: &str) -> Option<String> {
    let payload = trimmed.strip_prefix("from ")?;
    let module_name = payload.split_whitespace().next()?;
    if module_name.is_empty() {
        return None;
    }
    Some(module_name.to_string())
}

fn is_valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn triple_quote_start(trimmed: &str) -> Option<(&'static str, usize)> {
    let mut prefix_len = 0;
    for ch in trimmed.chars() {
        if matches!(ch, 'r' | 'R' | 'u' | 'U' | 'b' | 'B' | 'f' | 'F') {
            prefix_len += ch.len_utf8();
        } else {
            break;
        }
    }

    let rest = &trimmed[prefix_len..];
    if rest.starts_with("\"\"\"") {
        Some(("\"\"\"", prefix_len))
    } else if rest.starts_with("'''") {
        Some(("'''", prefix_len))
    } else {
        None
    }
}

pub(super) fn extract_module_docstring(source: &str) -> Option<(String, usize)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    // Skip leading blank lines and comment lines.
    while i < lines.len() {
        let t = lines[i].trim();
        if !t.is_empty() && !t.starts_with('#') {
            break;
        }
        i += 1;
    }

    if i >= lines.len() {
        return None;
    }

    let first = lines[i].trim();
    let (quote, quote_start) = triple_quote_start(first)?;

    let start_line = i + 1; // 1-indexed
    let after_open = &first[quote_start + quote.len()..];

    // Single-line docstring closed on the same line.
    if let Some(close_pos) = after_open.find(quote) {
        return Some((after_open[..close_pos].to_string(), start_line));
    }

    // Multi-line: accumulate until closing delimiter.
    let mut content = after_open.to_string();
    i += 1;
    while i < lines.len() {
        let line = lines[i];
        if let Some(close_pos) = line.find(quote) {
            content.push('\n');
            content.push_str(line[..close_pos].trim_end());
            return Some((content, start_line));
        }
        content.push('\n');
        content.push_str(line);
        i += 1;
    }

    None
}

pub(super) fn is_docstring_line(trimmed: &str) -> bool {
    triple_quote_start(trimmed).is_some()
}

pub(super) fn self_assignment_name(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("self.")?;
    let name_end = rest
        .chars()
        .position(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'))?;
    let name = &rest[..name_end];
    let tail = rest[name_end..].trim_start();
    if tail.starts_with('=') || tail.starts_with(':') {
        return Some(name);
    }
    None
}

pub(super) fn returns_body(trimmed: &str) -> bool {
    if let Some(rest) = trimmed.strip_prefix("return") {
        let expression = strip_inline_comment(rest).trim();
        if expression == "body" {
            return true;
        }
        if let Some(inner) = expression
            .strip_prefix('(')
            .and_then(|v| v.strip_suffix(')'))
        {
            return inner.trim() == "body";
        }
    }
    false
}

use crate::models::ValveFieldInfo;

/// Extract the field name from a Pydantic-style field declaration line.
/// Matches patterns like `IDENTIFIER: type` or `IDENTIFIER = value`.
/// Returns a borrowed slice from the input, no allocation needed.
fn extract_field_name(trimmed: &str) -> Option<&str> {
    // Must start with an identifier character
    let name_end = trimmed
        .chars()
        .position(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'))?;
    if name_end == 0 {
        return None;
    }
    let name = &trimmed[..name_end];
    let rest = trimmed[name_end..].trim_start();
    // Must be followed by `:` (type annotation) or `=` (assignment)
    if rest.starts_with(':') || rest.starts_with('=') {
        // Skip dunder names, `class`, `def`, etc.
        if name.starts_with("__") && name.ends_with("__") {
            return None;
        }
        Some(name)
    } else {
        None
    }
}

/// Accumulate a possibly-multiline statement starting at `start` until
/// parentheses are balanced. Returns the joined text and the number of
/// lines consumed.
fn accumulate_statement(lines: &[&str], start: usize) -> (String, usize) {
    let mut text = lines[start].trim().to_string();
    let mut consumed = 1;

    while paren_balance(&text) > 0 && start + consumed < lines.len() {
        text.push(' ');
        text.push_str(lines[start + consumed].trim());
        consumed += 1;
    }

    (text, consumed)
}

fn next_unescaped_quote(value: &str, start: usize, quote: char) -> Option<usize> {
    let mut escaped = false;
    for (offset, ch) in value[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some(start + offset);
        }
    }
    None
}

fn has_password_input_type(statement: &str) -> bool {
    let mut pos = 0;
    while pos < statement.len() {
        let mut iter = statement[pos..].char_indices();
        let Some((key_rel_start, quote)) = iter.find(|(_, ch)| *ch == '"' || *ch == '\'') else {
            break;
        };
        let key_start = pos + key_rel_start;
        let value_start = key_start + quote.len_utf8();
        let Some(key_end) = next_unescaped_quote(statement, value_start, quote) else {
            break;
        };
        let key = &statement[value_start..key_end];
        pos = key_end + quote.len_utf8();

        if !key.eq_ignore_ascii_case("type") {
            continue;
        }

        let mut rest = &statement[pos..];
        rest = rest.trim_start();
        if !rest.starts_with(':') {
            continue;
        }
        rest = rest[1..].trim_start();
        let mut chars = rest.chars();
        let Some(value_quote) = chars.next() else {
            continue;
        };
        if value_quote != '"' && value_quote != '\'' {
            continue;
        }
        let value_start_in_rest = value_quote.len_utf8();
        let Some(value_end_in_rest) = next_unescaped_quote(rest, value_start_in_rest, value_quote)
        else {
            continue;
        };
        let field_value = &rest[value_start_in_rest..value_end_in_rest];
        if field_value.eq_ignore_ascii_case("password") {
            return true;
        }
    }
    false
}

/// Parse Valves/UserValves class body to extract field definitions.
/// `lines` is the full file line slice; `class_line_idx` is the 0-indexed
/// line where the class definition appears.
pub(super) fn parse_valve_fields(lines: &[&str], class_line_idx: usize) -> Vec<ValveFieldInfo> {
    let mut fields = Vec::new();

    // Determine the body indent from the first non-blank line after the class def
    let body_indent = lines
        .iter()
        .skip(class_line_idx + 1)
        .find(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .map(|l| crate::util::count_indent(l));

    let Some(body_indent) = body_indent else {
        return fields;
    };

    if body_indent == 0 {
        return fields;
    }

    let mut i = class_line_idx + 1;
    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim();

        // Skip blank lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }

        let indent = crate::util::count_indent(raw);

        // Exited the class body
        if indent < body_indent {
            break;
        }

        // Only process lines at exactly the body indent level (skip nested)
        if indent > body_indent {
            i += 1;
            continue;
        }

        // Skip method definitions and nested classes
        if trimmed.starts_with("def ")
            || trimmed.starts_with("async def ")
            || trimmed.starts_with("class ")
        {
            i += 1;
            continue;
        }

        if let Some(name) = extract_field_name(trimmed) {
            let line_no = i + 1; // 1-indexed
            let (full_stmt, consumed) = accumulate_statement(lines, i);
            let has_password_type = has_password_input_type(&full_stmt);
            fields.push(ValveFieldInfo {
                name: name.to_ascii_lowercase(),
                line: line_no,
                has_password_type,
            });
            i += consumed;
        } else {
            i += 1;
        }
    }

    fields
}

#[cfg(test)]
mod tests {
    use super::{
        collect_function_signature, extract_module_docstring, parse_valve_fields, returns_body,
    };

    #[test]
    fn collect_function_signature_ignores_inline_comments_across_lines() {
        let lines = [
            "async def pipe(  # comment",
            "    self,",
            "    body: dict,  # trailing",
            ") -> dict:",
            "    return body",
        ];

        let (signature, consumed) = collect_function_signature(&lines, 0);
        assert_eq!(consumed, 4);
        assert_eq!(
            signature, "async def pipe( self, body: dict, ) -> dict:",
            "signature should be merged without comments"
        );
    }

    #[test]
    fn returns_body_allows_inline_comment() {
        assert!(returns_body("return body  # keep contract"));
    }

    #[test]
    fn returns_body_allows_parenthesized_body() {
        assert!(returns_body("return (body)"));
    }

    #[test]
    fn extract_module_docstring_supports_prefixed_triple_quotes() {
        let source = r#"r"""
title: Prefixed Header
version: 0.1.0
requirements: requests
"""
class Tools:
    pass
"#;

        let (docstring, line) =
            extract_module_docstring(source).expect("prefixed module docstring should be parsed");
        assert_eq!(line, 1);
        assert!(docstring.contains("title: Prefixed Header"));
        assert!(docstring.contains("version: 0.1.0"));
    }

    #[test]
    fn parse_valve_fields_requires_password_input_type_not_plain_password_word() {
        let source = [
            "class Valves(BaseModel):",
            "    api_key: str = Field(",
            "        default=\"\",",
            "        json_schema_extra={\"input\": {\"type\": \"text\", \"placeholder\": \"password\"}}",
            "    )",
        ];
        let fields = parse_valve_fields(&source, 0);
        assert_eq!(fields.len(), 1);
        assert!(
            !fields[0].has_password_type,
            "password keyword outside input.type should not count as masking"
        );
    }

    #[test]
    fn parse_valve_fields_detects_single_quoted_password_input_type() {
        let source = [
            "class Valves(BaseModel):",
            "    api_key: str = Field(default='', json_schema_extra={'input': {'type': 'password'}})",
        ];
        let fields = parse_valve_fields(&source, 0);
        assert_eq!(fields.len(), 1);
        assert!(
            fields[0].has_password_type,
            "single-quoted password input type should count as masking"
        );
    }
}
