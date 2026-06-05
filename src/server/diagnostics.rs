use std::str::FromStr;

use lsp_types::{
    CodeDescription, Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range, Uri,
};

use crate::models::{Issue, Severity};
use crate::rules::rule_doc;
use crate::server::position::{char_to_utf16, utf16_len};

/// The `source` field attached to every diagnostic we publish, so editors can
/// attribute findings to owui-lint and users can filter by it.
pub const DIAGNOSTIC_SOURCE: &str = "owui-lint";

/// Convert linter issues into LSP diagnostics. Issue positions are 1-indexed
/// (line and column); LSP positions are 0-indexed. We highlight from the issue
/// column to the end of the line so the squiggle is visible without needing a
/// precise token span (which the lightweight analyzer does not track).
pub fn issues_to_diagnostics(issues: &[Issue], source: &str) -> Vec<Diagnostic> {
    let lines: Vec<&str> = source.lines().collect();

    issues
        .iter()
        .map(|issue| issue_to_diagnostic(issue, &lines))
        .collect()
}

fn issue_to_diagnostic(issue: &Issue, lines: &[&str]) -> Diagnostic {
    let line_idx = issue.line.saturating_sub(1);
    let line = line_idx as u32;
    let line_text = lines.get(line_idx).copied().unwrap_or("");
    // The analyzer reports character-based columns; LSP wants UTF-16 units.
    let start_col = char_to_utf16(line_text, issue.column.saturating_sub(1));
    let line_end = utf16_len(line_text);
    // Ensure a non-empty range even on blank/short lines.
    let end_col = line_end.max(start_col + 1);

    let severity = match issue.severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
    };

    let code_description = rule_doc(issue.rule_id)
        .and_then(|doc| Uri::from_str(doc.help_url).ok())
        .map(|href| CodeDescription { href });

    Diagnostic {
        range: Range::new(Position::new(line, start_col), Position::new(line, end_col)),
        severity: Some(severity),
        code: Some(NumberOrString::String(issue.rule_id.to_string())),
        code_description,
        source: Some(DIAGNOSTIC_SOURCE.to_string()),
        message: issue.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn issue(rule_id: &'static str, line: usize, column: usize) -> Issue {
        Issue {
            rule_id,
            severity: Severity::Warning,
            message: "msg".to_string(),
            path: PathBuf::from("buffer.py"),
            line,
            column,
        }
    }

    #[test]
    fn converts_one_indexed_positions_to_zero_indexed() {
        // Source: line 1 is empty, line 2 has 9 chars ("def x():").
        let source = "\ndef x():\n";
        let diagnostics = issues_to_diagnostics(&[issue("OWT102", 2, 5)], source);
        let range = diagnostics[0].range;

        // 1-indexed (2,5) -> 0-indexed (1,4).
        assert_eq!(range.start, Position::new(1, 4));
        // Highlight runs to the end of the line (8 chars on "def x():").
        assert_eq!(range.end, Position::new(1, 8));
    }

    #[test]
    fn clamps_end_column_on_blank_or_short_line() {
        // Issue points past the end of a blank first line: ensure a non-empty
        // (at least 1-wide) range so the squiggle is visible.
        let diagnostics = issues_to_diagnostics(&[issue("OWT102", 1, 3)], "\n");
        let range = diagnostics[0].range;
        assert_eq!(range.start, Position::new(0, 2));
        assert_eq!(range.end, Position::new(0, 3));
        assert!(range.end.character > range.start.character);
    }

    #[test]
    fn maps_severity_and_sets_owui_source() {
        let mut err = issue("OWT102", 1, 1);
        err.severity = Severity::Error;
        let diagnostics = issues_to_diagnostics(&[err], "x\n");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostics[0].source.as_deref(), Some(DIAGNOSTIC_SOURCE));
        assert_eq!(
            diagnostics[0].code,
            Some(NumberOrString::String("OWT102".to_string()))
        );
    }

    #[test]
    fn columns_are_utf16_code_units_not_chars() {
        // The crab "🦀" is one char but two UTF-16 code units. An issue at
        // (char) column 4 — the `d` of `def`, after "🦀 x " — must map to UTF-16
        // column 5, and the line-end column must count the crab as two units.
        let source = "🦀 x def y():\n";
        let diagnostics = issues_to_diagnostics(&[issue("OWT102", 1, 5)], source);
        let range = diagnostics[0].range;
        // 1-indexed char column 5 -> 0-indexed char 4 -> UTF-16 offset 5.
        assert_eq!(range.start, Position::new(0, 5));
        // End runs to the line's UTF-16 length (one more than its char count).
        let utf16_len = "🦀 x def y():".encode_utf16().count() as u32;
        assert_eq!(range.end, Position::new(0, utf16_len));
    }

    #[test]
    fn unknown_rule_has_no_code_description() {
        let diagnostics = issues_to_diagnostics(&[issue("ZZZ999", 1, 1)], "x\n");
        assert!(diagnostics[0].code_description.is_none());
    }
}
