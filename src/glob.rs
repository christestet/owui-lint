pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let mut memo = vec![vec![None; t.len() + 1]; p.len() + 1];
    matches(&p, 0, &t, 0, &mut memo)
}

fn matches(
    pattern: &[char],
    pi: usize,
    text: &[char],
    ti: usize,
    memo: &mut [Vec<Option<bool>>],
) -> bool {
    if let Some(cached) = memo[pi][ti] {
        return cached;
    }

    let result = if pi == pattern.len() {
        ti == text.len()
    } else {
        let current = pattern[pi];
        if current == '*' {
            let is_double = pi + 1 < pattern.len() && pattern[pi + 1] == '*';
            if is_double {
                let skip_double_star = matches(pattern, pi + 2, text, ti, memo)
                    || (pi + 2 < pattern.len()
                        && pattern[pi + 2] == '/'
                        && matches(pattern, pi + 3, text, ti, memo));

                skip_double_star || (ti < text.len() && matches(pattern, pi, text, ti + 1, memo))
            } else {
                matches(pattern, pi + 1, text, ti, memo)
                    || (ti < text.len()
                        && text[ti] != '/'
                        && matches(pattern, pi, text, ti + 1, memo))
            }
        } else if current == '?' {
            ti < text.len() && text[ti] != '/' && matches(pattern, pi + 1, text, ti + 1, memo)
        } else {
            ti < text.len() && current == text[ti] && matches(pattern, pi + 1, text, ti + 1, memo)
        }
    };

    memo[pi][ti] = Some(result);
    result
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn supports_double_star() {
        assert!(glob_match("**/*.py", "a/b/c.py"));
        assert!(glob_match("**/*.py", "file.py"));
    }

    #[test]
    fn single_star_does_not_cross_slashes() {
        assert!(glob_match("*.py", "a.py"));
        assert!(!glob_match("*.py", "a/b.py"));
    }

    #[test]
    fn supports_question_mark() {
        assert!(glob_match("file?.py", "file1.py"));
        assert!(!glob_match("file?.py", "file10.py"));
    }
}
