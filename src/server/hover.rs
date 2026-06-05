use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};

use crate::models::Issue;
use crate::rules::rule_doc;
use crate::server::state::ServerState;

/// Provide hover text when the cursor is on a line carrying an owui-lint
/// finding: render the rule's documentation (title, summary, remediation,
/// docs link and minimum Open WebUI version) as Markdown.
pub fn handle_hover(state: &ServerState, params: &HoverParams) -> Option<Hover> {
    let position = &params.text_document_position_params;
    let uri = &position.text_document.uri;
    let document = state.document(uri)?;

    // Read the cached lint result so hover always agrees with the published
    // diagnostics for this document version.
    let target_line = position.position.line as usize + 1;
    let issue = document
        .issues
        .iter()
        .find(|issue| issue.line == target_line)?;

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: render_issue(issue),
        }),
        range: None,
    })
}

fn render_issue(issue: &Issue) -> String {
    match rule_doc(issue.rule_id) {
        Some(doc) => format!(
            "**{title}** · `{id}`\n\n{summary}\n\n**Fix:** {remediation}\n\n[Documentation]({url}) · Open WebUI ≥ {version}",
            title = doc.title,
            id = doc.id,
            summary = doc.summary,
            remediation = doc.remediation,
            url = doc.help_url,
            version = doc.openwebui_version,
        ),
        None => format!("`{}`\n\n{}", issue.rule_id, issue.message),
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::server::state::ServerState;
    use lsp_types::{
        Position, TextDocumentIdentifier, TextDocumentPositionParams, Uri, WorkDoneProgressParams,
    };

    // `def search` (line 2, 1-indexed) triggers OWT101/OWT102; `return query`
    // (line 3) carries no finding.
    const TOOLS_SOURCE: &str = "class Tools:\n    def search(self, query):\n        return query\n";

    fn hover_at(source: &str, line: u32) -> Option<Hover> {
        let uri = Uri::from_str("file:///tmp/owui_hover_test.py").expect("valid uri");
        let mut state = ServerState::new(None);
        state.upsert(uri.clone(), source.to_string(), 1);

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position::new(line, 0),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        handle_hover(&state, &params)
    }

    #[test]
    fn hover_on_finding_line_renders_rule_doc() {
        // 0-indexed line 1 == the `def search` line (1-indexed line 2).
        let hover = hover_at(TOOLS_SOURCE, 1).expect("hover present on finding line");
        let HoverContents::Markup(content) = hover.contents else {
            panic!("expected markup hover contents");
        };
        assert_eq!(content.kind, MarkupKind::Markdown);
        assert!(
            content.value.contains("OWT101"),
            "hover should mention the rule id, got: {}",
            content.value
        );
    }

    #[test]
    fn hover_off_finding_line_is_none() {
        // 0-indexed line 2 == `return query` (1-indexed line 3): no finding.
        assert!(hover_at(TOOLS_SOURCE, 2).is_none());
    }

    #[test]
    fn hover_none_for_unknown_document() {
        let uri = Uri::from_str("file:///tmp/owui_hover_missing.py").expect("valid uri");
        let state = ServerState::new(None);
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position::new(0, 0),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        assert!(handle_hover(&state, &params).is_none());
    }
}
