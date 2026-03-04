use crate::models::SyntaxErrorInfo;

struct StringState {
    quote: Option<char>,
    triple_quote: bool,
    escaped: bool,
}

fn advance_through_string(
    state: &mut StringState,
    chars: &[char],
    idx: usize,
    column: &mut usize,
) -> usize {
    let active_quote = match state.quote {
        Some(q) => q,
        None => return idx,
    };
    let ch = chars[idx];

    if state.escaped {
        state.escaped = false;
        return idx + 1;
    }

    if ch == '\\' {
        state.escaped = true;
        return idx + 1;
    }

    if state.triple_quote {
        if ch == active_quote
            && idx + 2 < chars.len()
            && chars[idx + 1] == active_quote
            && chars[idx + 2] == active_quote
        {
            state.quote = None;
            state.triple_quote = false;
            *column += 2;
            return idx + 3;
        }
        return idx + 1;
    }

    if ch == active_quote {
        state.quote = None;
    }
    idx + 1
}

fn check_closing_bracket(
    ch: char,
    stack: &mut Vec<(char, usize, usize)>,
    source: &str,
    line: usize,
    column: usize,
) -> Option<SyntaxErrorInfo> {
    let expected_open = match ch {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        _ => unreachable!(),
    };
    if let Some((open, _, _)) = stack.pop() {
        if open != expected_open {
            return Some(SyntaxErrorInfo {
                message: format!("Mismatched delimiter: found '{ch}'"),
                line,
                column,
                text: source
                    .lines()
                    .nth(line - 1)
                    .map(|line_text| line_text.to_string()),
            });
        }
    } else {
        return Some(SyntaxErrorInfo {
            message: format!("Unmatched closing delimiter: '{ch}'"),
            line,
            column,
            text: source
                .lines()
                .nth(line - 1)
                .map(|line_text| line_text.to_string()),
        });
    }
    None
}

pub(super) fn detect_syntax_error(source: &str) -> Option<SyntaxErrorInfo> {
    let mut stack: Vec<(char, usize, usize)> = Vec::new();
    let mut line = 1;
    let mut column = 0;
    let chars: Vec<char> = source.chars().collect();
    let mut idx = 0;

    let mut string_state = StringState {
        quote: None,
        triple_quote: false,
        escaped: false,
    };

    while idx < chars.len() {
        let ch = chars[idx];
        if ch == '\n' {
            line += 1;
            column = 0;
            string_state.escaped = false;
            idx += 1;
            continue;
        }

        column += 1;

        if string_state.quote.is_some() {
            idx = advance_through_string(&mut string_state, &chars, idx, &mut column);
            continue;
        }

        if ch == '#' {
            while idx < chars.len() && chars[idx] != '\n' {
                idx += 1;
            }
            continue;
        }

        if ch == '\'' || ch == '"' {
            let is_triple = idx + 2 < chars.len() && chars[idx + 1] == ch && chars[idx + 2] == ch;
            string_state.quote = Some(ch);
            string_state.triple_quote = is_triple;
            if is_triple {
                idx += 3;
                column += 2;
            } else {
                idx += 1;
            }
            continue;
        }

        if matches!(ch, '(' | '[' | '{') {
            stack.push((ch, line, column));
            idx += 1;
            continue;
        }

        if matches!(ch, ')' | ']' | '}') {
            if let Some(error) = check_closing_bracket(ch, &mut stack, source, line, column) {
                return Some(error);
            }
            idx += 1;
            continue;
        }

        idx += 1;
    }

    if let Some((open, open_line, open_col)) = stack.pop() {
        return Some(SyntaxErrorInfo {
            message: format!("Unclosed delimiter: '{open}'"),
            line: open_line,
            column: open_col,
            text: source
                .lines()
                .nth(open_line.saturating_sub(1))
                .map(|line_text| line_text.to_string()),
        });
    }

    if let Some(quote) = string_state.quote {
        return Some(SyntaxErrorInfo {
            message: format!("Unclosed string literal: '{quote}'"),
            line,
            column,
            text: source
                .lines()
                .nth(line.saturating_sub(1))
                .map(|line_text| line_text.to_string()),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::detect_syntax_error;

    #[test]
    fn detects_unclosed_double_quoted_string() {
        let source = "x = \"hello\n";
        let err = detect_syntax_error(source).expect("should detect unclosed string");
        assert!(err.message.contains("Unclosed string literal"));
    }

    #[test]
    fn ignores_delimiters_inside_strings() {
        let source = "value = \"[(not real)]\"\n";
        assert!(
            detect_syntax_error(source).is_none(),
            "delimiters inside strings should be ignored"
        );
    }
}
