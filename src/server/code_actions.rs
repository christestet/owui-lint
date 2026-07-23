use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::{
    CodeAction, CodeActionOrCommand, CodeActionParams, CodeActionResponse, Command, Diagnostic,
    NumberOrString, Position, Range, TextEdit, Uri, WorkspaceEdit,
};

use crate::server::diagnostics::DIAGNOSTIC_SOURCE;
use crate::server::position::byte_to_utf16;
use crate::server::state::ServerState;

/// Command id the client invokes (via `workspace/executeCommand`) to persist a
/// rule override. The server then writes the config file and refreshes
/// diagnostics. Registered in the server's `executeCommandProvider`.
pub const DISABLE_RULE_COMMAND: &str = "owui-lint.disableRule";

/// Build quick-fixes for the owui-lint diagnostics overlapping the requested
/// range. Each fix is derived purely from the diagnostic's rule code and the
/// document text, so no extra analysis state is required.
pub fn handle_code_action(
    state: &ServerState,
    params: &CodeActionParams,
) -> Option<CodeActionResponse> {
    let uri = &params.text_document.uri;
    let document = state.document(uri)?;
    let lines: Vec<&str> = document.text.lines().collect();

    let mut actions: Vec<CodeActionOrCommand> = Vec::new();

    for diagnostic in &params.context.diagnostics {
        let Some(rule_id) = owui_rule_id(diagnostic) else {
            continue;
        };

        if let Some(action) = fix_for_rule(uri, &lines, diagnostic, rule_id) {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        // Always offer to silence the rule project-wide.
        actions.push(CodeActionOrCommand::CodeAction(disable_rule_action(
            diagnostic, rule_id,
        )));
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

/// Extract the rule id from a diagnostic that owui-lint produced.
fn owui_rule_id(diagnostic: &Diagnostic) -> Option<&str> {
    if diagnostic.source.as_deref() != Some(DIAGNOSTIC_SOURCE) {
        return None;
    }
    match &diagnostic.code {
        Some(NumberOrString::String(code)) => Some(code.as_str()),
        _ => None,
    }
}

fn fix_for_rule(
    uri: &Uri,
    lines: &[&str],
    diagnostic: &Diagnostic,
    rule_id: &str,
) -> Option<CodeAction> {
    let line_idx = diagnostic.range.start.line as usize;
    match rule_id {
        // Make a handler async by inserting `async ` before `def`.
        "OWT102" | "OWP202" | "OWA401" | "OWE601" => async_fix(uri, lines, diagnostic, line_idx),
        // Insert a docstring stub as the first statement of the method body.
        "OWT101" => docstring_fix(uri, lines, diagnostic, line_idx),
        // Add a missing module-header field inside the module docstring.
        "OWUI030" => header_field_fix(uri, lines, diagnostic, line_idx, "version: 0.1.0"),
        "OWUI032" => header_field_fix(uri, lines, diagnostic, line_idx, "title: My Extension"),
        _ => None,
    }
}

fn async_fix(
    uri: &Uri,
    lines: &[&str],
    diagnostic: &Diagnostic,
    line_idx: usize,
) -> Option<CodeAction> {
    let line = lines.get(line_idx)?;
    let def_col = byte_to_utf16(line, line.find("def ")?);
    let position = Position::new(line_idx as u32, def_col);
    let edit = TextEdit {
        range: Range::new(position, position),
        new_text: "async ".to_string(),
    };
    Some(quickfix(uri, "Make method async", diagnostic, edit))
}

fn docstring_fix(
    uri: &Uri,
    lines: &[&str],
    diagnostic: &Diagnostic,
    line_idx: usize,
) -> Option<CodeAction> {
    // Find the end of the (possibly multi-line) signature: the line whose
    // trimmed content ends with `:`.
    let mut end_idx = line_idx;
    while end_idx < lines.len() && !lines[end_idx].trim_end().ends_with(':') {
        end_idx += 1;
    }
    if end_idx >= lines.len() {
        return None;
    }

    let signature_indent = lines[line_idx]
        .chars()
        .take_while(|c| c.is_whitespace())
        .count();
    let body_indent = " ".repeat(signature_indent + 4);
    let insert_at = Position::new(end_idx as u32 + 1, 0);
    let edit = TextEdit {
        range: Range::new(insert_at, insert_at),
        new_text: format!("{body_indent}\"\"\"TODO: describe what this tool does.\"\"\"\n"),
    };
    Some(quickfix(uri, "Add docstring stub", diagnostic, edit))
}

fn header_field_fix(
    uri: &Uri,
    lines: &[&str],
    diagnostic: &Diagnostic,
    docstring_line_idx: usize,
    field: &str,
) -> Option<CodeAction> {
    let opening = lines.get(docstring_line_idx)?;
    let quote = if opening.contains("\"\"\"") {
        "\"\"\""
    } else if opening.contains("'''") {
        "'''"
    } else {
        return None;
    };
    let title = format!("Add `{field}` to module header");

    // Single-line docstring (e.g. `"""title: x"""`): the opening and closing
    // quotes share a line, so inserting on the *next* line would place the field
    // outside the docstring. Instead, expand it to a multi-line docstring by
    // inserting the field on its own line just before the closing quotes.
    if opening.matches(quote).count() >= 2 {
        let close_col = byte_to_utf16(opening, opening.rfind(quote)?);
        let indent: String = opening.chars().take_while(|c| c.is_whitespace()).collect();
        let position = Position::new(docstring_line_idx as u32, close_col);
        let edit = TextEdit {
            range: Range::new(position, position),
            new_text: format!("\n{indent}{field}\n{indent}"),
        };
        return Some(quickfix(uri, &title, diagnostic, edit));
    }

    // Multi-line docstring: insert the field on the line right after the opening.
    let insert_at = Position::new(docstring_line_idx as u32 + 1, 0);
    let edit = TextEdit {
        range: Range::new(insert_at, insert_at),
        new_text: format!("{field}\n"),
    };
    Some(quickfix(uri, &title, diagnostic, edit))
}

fn quickfix(uri: &Uri, title: &str, diagnostic: &Diagnostic, edit: TextEdit) -> CodeAction {
    let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);
    CodeAction {
        title: title.to_string(),
        kind: Some(lsp_types::CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    }
}

fn disable_rule_action(diagnostic: &Diagnostic, rule_id: &str) -> CodeAction {
    CodeAction {
        title: format!("Disable {rule_id} for this project"),
        kind: Some(lsp_types::CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: None,
        command: Some(Command {
            title: format!("Disable {rule_id}"),
            command: DISABLE_RULE_COMMAND.to_string(),
            arguments: Some(vec![serde_json::Value::String(rule_id.to_string())]),
        }),
        is_preferred: Some(false),
        disabled: None,
        data: None,
    }
}

/// Persist `RULE: off` to the workspace config file (creating `config.yml` when
/// none exists). The line-based config loader tolerates an appended `rules:`
/// block even if one already exists, keeping this write simple and robust.
pub fn disable_rule_in_config(root: &Path, rule_id: &str) -> std::io::Result<PathBuf> {
    let target = [
        "config.yml",
        "config.yaml",
        "owui-lint.yml",
        "owui-lint.yaml",
    ]
    .iter()
    .map(|name| root.join(name))
    .find(|path| path.exists())
    .unwrap_or_else(|| root.join("config.yml"));

    let mut content = std::fs::read_to_string(&target).unwrap_or_default();
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!("rules:\n  {rule_id}: off\n"));
    std::fs::write(&target, content)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use lsp_types::{
        CodeActionContext, CodeActionParams, NumberOrString, PartialResultParams,
        TextDocumentIdentifier, WorkDoneProgressParams,
    };

    fn uri() -> Uri {
        Uri::from_str("file:///tmp/owui_code_action_test.py").expect("valid uri")
    }

    /// Build an owui-lint diagnostic for `rule_id` anchored at `line` (0-indexed).
    fn diagnostic(rule_id: &str, line: u32) -> Diagnostic {
        Diagnostic {
            range: Range::new(Position::new(line, 0), Position::new(line, 1)),
            source: Some(DIAGNOSTIC_SOURCE.to_string()),
            code: Some(NumberOrString::String(rule_id.to_string())),
            message: String::new(),
            ..Diagnostic::default()
        }
    }

    fn state_with(text: &str) -> ServerState {
        let mut state = ServerState::new(None);
        state.upsert(uri(), text.to_string(), 1);
        state
    }

    fn code_action_params(diagnostics: Vec<Diagnostic>) -> CodeActionParams {
        let range = diagnostics
            .first()
            .map(|d| d.range)
            .unwrap_or_else(|| Range::new(Position::new(0, 0), Position::new(0, 0)));
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            range,
            context: CodeActionContext {
                diagnostics,
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Pull the single TextEdit from a quick-fix titled `title`, if present.
    fn edit_for(actions: &[CodeActionOrCommand], title: &str) -> Option<TextEdit> {
        actions.iter().find_map(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return None;
            };
            if action.title != title {
                return None;
            }
            let edits = action.edit.as_ref()?.changes.as_ref()?.get(&uri())?;
            edits.first().cloned()
        })
    }

    #[test]
    fn async_fix_inserts_async_before_def() {
        let source = "class Tools:\n    def search(self, query):\n        return query\n";
        let state = state_with(source);
        // `def search` is on line index 1, indented by 4 spaces.
        let params = code_action_params(vec![diagnostic("OWT102", 1)]);

        let actions = handle_code_action(&state, &params).expect("actions present");
        let edit = edit_for(&actions, "Make method async").expect("async fix present");

        assert_eq!(edit.new_text, "async ");
        // Inserted exactly at the `def` column (4), zero-width.
        assert_eq!(edit.range.start, Position::new(1, 4));
        assert_eq!(edit.range.end, Position::new(1, 4));
    }

    #[test]
    fn header_field_fix_multiline_inserts_inside_docstring() {
        let source = "\"\"\"\ntitle: My Tool\n\"\"\"\n";
        let state = state_with(source);
        // OWUI030 (missing version) anchored at the opening quote line (index 0).
        let params = code_action_params(vec![diagnostic("OWUI030", 0)]);

        let actions = handle_code_action(&state, &params).expect("actions present");
        let edit = edit_for(&actions, "Add `version: 0.1.0` to module header")
            .expect("header fix present");

        // Inserted at the start of line 1 — between the opening quote and content.
        assert_eq!(edit.range.start, Position::new(1, 0));
        assert_eq!(edit.new_text, "version: 0.1.0\n");
    }

    #[test]
    fn header_field_fix_singleline_expands_to_multiline() {
        // Regression: a single-line docstring must not get the field appended on
        // the line *after* the (already closed) docstring.
        let source = "\"\"\"title: My Tool\"\"\"\n";
        let state = state_with(source);
        let params = code_action_params(vec![diagnostic("OWUI030", 0)]);

        let actions = handle_code_action(&state, &params).expect("actions present");
        let edit = edit_for(&actions, "Add `version: 0.1.0` to module header")
            .expect("header fix present");

        // The edit must be applied on the docstring line (index 0), before the
        // closing quotes — not on the next line.
        assert_eq!(edit.range.start.line, 0);
        let close_col = source.rfind("\"\"\"").expect("closing quotes") as u32;
        assert_eq!(edit.range.start.character, close_col);

        // Applying the edit (a single-line, zero-width insertion at close_col)
        // yields a docstring that still encloses the new field.
        let mut patched = source.to_string();
        patched.insert_str(close_col as usize, &edit.new_text);
        let field_pos = patched.find("version: 0.1.0").expect("field inserted");
        let close_pos = patched.rfind("\"\"\"").expect("closing quotes remain");
        assert!(
            field_pos < close_pos,
            "field must sit inside the docstring: {patched:?}"
        );
    }

    #[test]
    fn disable_action_always_offered_with_command() {
        let source = "class Tools:\n    def search(self, query):\n        return query\n";
        let state = state_with(source);
        let params = code_action_params(vec![diagnostic("OWT102", 1)]);

        let actions = handle_code_action(&state, &params).expect("actions present");
        let disable = actions
            .iter()
            .find_map(|action| match action {
                CodeActionOrCommand::CodeAction(a)
                    if a.title == "Disable OWT102 for this project" =>
                {
                    a.command.clone()
                }
                _ => None,
            })
            .expect("disable action present");

        assert_eq!(disable.command, DISABLE_RULE_COMMAND);
        assert_eq!(
            disable.arguments,
            Some(vec![serde_json::Value::String("OWT102".to_string())])
        );
    }

    #[test]
    fn non_owui_diagnostics_yield_no_actions() {
        let state = state_with("class Tools:\n    def search(self):\n        pass\n");
        let mut foreign = diagnostic("OWT102", 1);
        foreign.source = Some("pyright".to_string()); // not ours
        let params = code_action_params(vec![foreign]);

        assert!(handle_code_action(&state, &params).is_none());
    }

    #[test]
    fn disable_rule_in_config_creates_file_when_missing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let written = disable_rule_in_config(dir.path(), "OWT102").expect("write config");

        assert_eq!(written, dir.path().join("config.yml"));
        let contents = std::fs::read_to_string(&written).expect("read config");
        assert!(contents.contains("OWT102: off"), "got: {contents:?}");
    }

    #[test]
    fn disable_rule_in_config_appends_to_existing_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let existing = dir.path().join("config.yaml");
        std::fs::write(&existing, "lint:\n  include:\n    - \"**/*.py\"").expect("seed config");

        let written = disable_rule_in_config(dir.path(), "OWP202").expect("write config");

        // Picks the existing config.yaml rather than creating a new config.yml.
        assert_eq!(written, existing);
        let contents = std::fs::read_to_string(&written).expect("read config");
        assert!(
            contents.contains("**/*.py"),
            "original content kept: {contents:?}"
        );
        assert!(
            contents.contains("OWP202: off"),
            "rule appended: {contents:?}"
        );
        assert!(contents.contains("\n"), "newline boundary added");
    }
}
