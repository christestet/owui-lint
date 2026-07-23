use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, Documentation,
    InsertTextFormat, MarkupContent, MarkupKind,
};

use crate::analysis::analyze_source;
use crate::server::state::ServerState;

/// Provide Open WebUI scaffolding snippets. These intentionally do NOT duplicate
/// general Python completion (Pylance/Pyright handle that); they only offer the
/// boilerplate that is specific to Open WebUI extensions. The set is chosen by a
/// cheap heuristic: the current line's indentation and whether the cursor sits
/// inside the module docstring header.
pub fn handle_completion(
    state: &ServerState,
    params: &CompletionParams,
) -> Option<CompletionResponse> {
    let position = &params.text_document_position;
    let uri = &position.text_document.uri;
    let document = state.document(uri)?;

    let line_idx = position.position.line as usize;
    let lines: Vec<&str> = document.text.lines().collect();
    let current_line = lines.get(line_idx).copied().unwrap_or("");
    let indent = current_line
        .chars()
        .take_while(|c| c.is_whitespace())
        .count();

    let items = if in_module_docstring(&document.text, line_idx) {
        header_field_items()
    } else if indent == 0 {
        class_skeleton_items()
    } else {
        class_member_items()
    };

    Some(CompletionResponse::Array(items))
}

/// Heuristic: is `line_idx` within the module docstring block? We reuse the
/// analyzer to locate the docstring start, then scan for its closing triple
/// quote. Good enough to surface header-field snippets while editing the header.
fn in_module_docstring(source: &str, line_idx: usize) -> bool {
    let module = analyze_source(std::path::Path::new("buffer.py"), source);
    let Some(start_line) = module.module_docstring_line else {
        return false;
    };
    let start_idx = start_line.saturating_sub(1);
    if line_idx < start_idx {
        return false;
    }

    let lines: Vec<&str> = source.lines().collect();
    let Some(opening) = lines.get(start_idx) else {
        return false;
    };
    let quote = if opening.contains("\"\"\"") {
        "\"\"\""
    } else if opening.contains("'''") {
        "'''"
    } else {
        return false;
    };

    // Single-line docstring (opening and closing on the same line).
    if opening.matches(quote).count() >= 2 {
        return line_idx == start_idx;
    }

    for (offset, line) in lines.iter().enumerate().skip(start_idx + 1) {
        if line.contains(quote) {
            return line_idx >= start_idx && line_idx <= offset;
        }
    }
    // Unterminated docstring: treat everything from the start onward as inside.
    line_idx >= start_idx
}

fn snippet(label: &str, detail: &str, body: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some(detail.to_string()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```python\n{body}\n```"),
        })),
        insert_text: Some(body.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..CompletionItem::default()
    }
}

fn header_field_items() -> Vec<CompletionItem> {
    vec![
        snippet("version", "Open WebUI header field", "version: ${1:0.1.0}"),
        snippet(
            "title",
            "Open WebUI header field",
            "title: ${1:My Extension}",
        ),
        snippet(
            "requirements",
            "Open WebUI header field",
            "requirements: ${1:package>=1.0.0}",
        ),
        snippet(
            "author",
            "Open WebUI header field",
            "author: ${1:Your Name}",
        ),
        snippet(
            "description",
            "Open WebUI header field",
            "description: ${1:What this extension does}",
        ),
    ]
}

fn class_skeleton_items() -> Vec<CompletionItem> {
    vec![
        snippet(
            "owui-tools",
            "Open WebUI Tools skeleton",
            "class Tools:\n    class Valves(BaseModel):\n        ${1:api_key}: str = \"\"\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def ${2:do_something}(self, ${3:query}: str) -> str:\n        \"\"\"${4:Describe what this tool does.}\"\"\"\n        return ${5:\"\"}",
        ),
        snippet(
            "owui-pipe",
            "Open WebUI Pipe skeleton",
            "class Pipe:\n    class Valves(BaseModel):\n        ${1:model}: str = \"\"\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def pipe(self, body: dict) -> ${2:str}:\n        ${0:return body}",
        ),
        snippet(
            "owui-filter",
            "Open WebUI Filter skeleton",
            "class Filter:\n    class Valves(BaseModel):\n        ${1:priority}: int = 0\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def inlet(self, body: dict) -> dict:\n        ${2:return body}\n\n    async def outlet(self, body: dict) -> dict:\n        ${0:return body}",
        ),
        snippet(
            "owui-action",
            "Open WebUI Action skeleton",
            "class Action:\n    class Valves(BaseModel):\n        ${1:enabled}: bool = True\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def action(self, body: dict) -> ${2:dict}:\n        ${0:return body}",
        ),
        snippet(
            "owui-pipeline",
            "Open WebUI Pipeline skeleton",
            "class Pipeline:\n    def __init__(self):\n        self.name = \"${1:My Pipeline}\"\n\n    async def pipe(self, body: dict) -> ${2:str}:\n        ${0:return body}",
        ),
        snippet(
            "owui-event",
            "Open WebUI Event skeleton",
            "class Event:\n    class Valves(BaseModel):\n        ${1:enabled}: bool = True\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def event(self, event: dict, __event_name__: str) -> ${2:None}:\n        ${0:pass}",
        ),
    ]
}

fn class_member_items() -> Vec<CompletionItem> {
    vec![
        snippet(
            "Valves",
            "Pydantic configuration class",
            "class Valves(BaseModel):\n    ${1:api_key}: str = \"\"",
        ),
        snippet(
            "UserValves",
            "Per-user Pydantic configuration class",
            "class UserValves(BaseModel):\n    ${1:enabled}: bool = True",
        ),
        snippet(
            "pipe-method",
            "Pipe handler method",
            "async def pipe(self, body: dict) -> ${1:str}:\n    ${0:return body}",
        ),
        snippet(
            "inlet-method",
            "Filter inlet method",
            "async def inlet(self, body: dict) -> dict:\n    ${0:return body}",
        ),
        snippet(
            "outlet-method",
            "Filter outlet method",
            "async def outlet(self, body: dict) -> dict:\n    ${0:return body}",
        ),
        snippet(
            "action-method",
            "Action handler method",
            "async def action(self, body: dict) -> ${1:dict}:\n    ${0:return body}",
        ),
        snippet(
            "event-method",
            "Event handler method",
            "async def event(self, event: dict, __event_name__: str) -> ${1:None}:\n    ${0:pass}",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use lsp_types::{
        CompletionParams, PartialResultParams, Position, TextDocumentIdentifier,
        TextDocumentPositionParams, Uri, WorkDoneProgressParams,
    };

    const MULTILINE_DOC: &str =
        "\"\"\"\ntitle: Tool\nversion: 0.1.0\n\"\"\"\n\nclass Tools:\n    pass\n";
    const SINGLELINE_DOC: &str = "\"\"\"title: Tool\"\"\"\n\nclass Tools:\n    pass\n";

    #[test]
    fn in_module_docstring_multiline() {
        // Lines 0..=3 are the docstring (open, two fields, close); line 5 is code.
        assert!(in_module_docstring(MULTILINE_DOC, 0));
        assert!(in_module_docstring(MULTILINE_DOC, 1));
        assert!(in_module_docstring(MULTILINE_DOC, 3));
        assert!(!in_module_docstring(MULTILINE_DOC, 5));
    }

    #[test]
    fn in_module_docstring_singleline_only_its_own_line() {
        assert!(in_module_docstring(SINGLELINE_DOC, 0));
        assert!(!in_module_docstring(SINGLELINE_DOC, 1));
        assert!(!in_module_docstring(SINGLELINE_DOC, 2));
    }

    #[test]
    fn in_module_docstring_unterminated_is_not_recognized() {
        // An unterminated triple-quote is a syntax error, so the analyzer does
        // not report a module docstring and no header snippets are offered.
        let src = "\"\"\"\ntitle: Tool\nstill inside\n";
        assert!(!in_module_docstring(src, 0));
        assert!(!in_module_docstring(src, 2));
    }

    #[test]
    fn in_module_docstring_false_without_docstring() {
        let src = "class Tools:\n    pass\n";
        assert!(!in_module_docstring(src, 0));
        assert!(!in_module_docstring(src, 1));
    }

    fn completion_at(source: &str, line: u32) -> Vec<CompletionItem> {
        let uri = Uri::from_str("file:///tmp/owui_completion_test.py").expect("valid uri");
        let mut state = ServerState::new(None);
        state.upsert(uri.clone(), source.to_string(), 1);

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position::new(line, 0),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };
        match handle_completion(&state, &params) {
            Some(CompletionResponse::Array(items)) => items,
            other => panic!("expected array response, got {other:?}"),
        }
    }

    #[test]
    fn completion_offers_header_fields_inside_docstring() {
        let labels: Vec<String> = completion_at(MULTILINE_DOC, 1)
            .into_iter()
            .map(|item| item.label)
            .collect();
        assert!(labels.iter().any(|l| l == "version"), "got {labels:?}");
        assert!(labels.iter().any(|l| l == "title"), "got {labels:?}");
    }

    #[test]
    fn completion_offers_class_skeletons_at_top_level() {
        let labels: Vec<String> = completion_at("\n", 0)
            .into_iter()
            .map(|item| item.label)
            .collect();
        assert!(labels.iter().any(|l| l == "owui-tools"), "got {labels:?}");
        assert!(labels.iter().any(|l| l == "owui-pipe"), "got {labels:?}");
        assert!(labels.iter().any(|l| l == "owui-event"), "got {labels:?}");
    }

    #[test]
    fn completion_offers_members_when_indented() {
        // Inside a class body (indented), offer member snippets, not skeletons.
        let source = "class Tools:\n    \n";
        let labels: Vec<String> = completion_at(source, 1)
            .into_iter()
            .map(|item| item.label)
            .collect();
        assert!(labels.iter().any(|l| l == "Valves"), "got {labels:?}");
        assert!(!labels.iter().any(|l| l == "owui-tools"), "got {labels:?}");
    }

    #[test]
    fn completion_none_for_unknown_document() {
        let uri = Uri::from_str("file:///tmp/owui_unknown.py").expect("valid uri");
        let state = ServerState::new(None);
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position::new(0, 0),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };
        assert!(handle_completion(&state, &params).is_none());
    }
}
