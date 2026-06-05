use std::collections::HashMap;
use std::path::Path;
use std::thread::JoinHandle;
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    Diagnostic, DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    ExecuteCommandParams, InitializeParams, NumberOrString, PublishDiagnosticsParams,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, Url,
    VersionedTextDocumentIdentifier,
};

const TOOLS_SOURCE: &str = "class Tools:\n    def search(self, query):\n        return query\n";
/// A corrected Tools class: the method is async and documented, so the non-async
/// (OWT102) and missing-docstring (OWT101) findings should disappear.
const TOOLS_SOURCE_FIXED: &str = "class Tools:\n    async def search(self, query):\n        \"\"\"Search.\"\"\"\n        return query\n";

/// Drive the language server end-to-end through an in-memory connection:
/// initialize, open a Tools buffer, assert the published diagnostics, and
/// request a quick-fix for the non-async finding.
#[test]
fn server_publishes_diagnostics_and_offers_quick_fix() {
    let (server_conn, client_conn) = Connection::memory();
    let server = std::thread::spawn(move || {
        owui_lint::server::serve(&server_conn).expect("server should run cleanly");
    });

    // --- initialize handshake ---
    let init_id = RequestId::from(1);
    send_request(
        &client_conn,
        init_id.clone(),
        "initialize",
        InitializeParams::default(),
    );
    recv_response(&client_conn, &init_id);
    send_notification(&client_conn, "initialized", serde_json::json!({}));

    // --- open a Tools document ---
    let uri = Url::parse("file:///tmp/owui_lsp_test_tools.py").expect("valid uri");
    send_notification(
        &client_conn,
        "textDocument/didOpen",
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "python".to_string(),
                version: 1,
                text: TOOLS_SOURCE.to_string(),
            },
        },
    );

    let diagnostics = recv_publish_diagnostics(&client_conn);
    let codes: Vec<String> = diagnostics.iter().filter_map(diagnostic_code).collect();
    assert!(
        codes.iter().any(|code| code == "OWT101"),
        "expected missing-docstring finding, got {codes:?}"
    );
    assert!(
        codes.iter().any(|code| code == "OWT102"),
        "expected non-async finding, got {codes:?}"
    );

    // --- request a quick-fix for the non-async (OWT102) finding ---
    let owt102 = diagnostics
        .iter()
        .find(|diag| diagnostic_code(diag).as_deref() == Some("OWT102"))
        .expect("OWT102 diagnostic present")
        .clone();

    let action_id = RequestId::from(2);
    send_request(
        &client_conn,
        action_id.clone(),
        "textDocument/codeAction",
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: owt102.range,
            context: CodeActionContext {
                diagnostics: vec![owt102],
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
    );
    let response = recv_response(&client_conn, &action_id);
    let actions: CodeActionResponse =
        serde_json::from_value(response).expect("code action response");
    let titles: Vec<String> = actions
        .iter()
        .map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action.title.clone(),
            CodeActionOrCommand::Command(command) => command.title.clone(),
        })
        .collect();
    assert!(
        titles.iter().any(|title| title == "Make method async"),
        "expected an async quick-fix, got {titles:?}"
    );
    assert!(
        titles.iter().any(|title| title.contains("Disable OWT102")),
        "expected a disable-rule action, got {titles:?}"
    );

    // The async quick-fix must carry a real edit that inserts `async ` at the
    // `def` column on the method line (not merely an action with the right title).
    let async_action = actions
        .iter()
        .find_map(|action| match action {
            CodeActionOrCommand::CodeAction(a) if a.title == "Make method async" => Some(a),
            _ => None,
        })
        .expect("async code action present");
    let edit = first_text_edit(async_action, &uri);
    assert_eq!(edit.new_text, "async ");
    // `def search` is indented by 4 spaces on line index 1.
    assert_eq!(edit.range.start.line, 1);
    assert_eq!(edit.range.start.character, 4);
    assert_eq!(
        edit.range.start, edit.range.end,
        "edit is a zero-width insert"
    );

    shutdown(&client_conn, server, 3);
}

/// Disabling a rule via `workspace/executeCommand` must persist the override to
/// the workspace config file and re-publish diagnostics for every open document
/// without the silenced rule. Exercises the full executeCommand round trip and
/// multi-document refresh.
#[test]
fn execute_command_disables_rule_and_refreshes_open_documents() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let (client, server) = spawn_server();
    handshake(&client, Some(workspace.path()));

    // Open two Tools documents; both report OWT102.
    let uri_a = Url::parse("file:///tmp/owui_cmd_a.py").expect("uri a");
    let uri_b = Url::parse("file:///tmp/owui_cmd_b.py").expect("uri b");
    open_document(&client, &uri_a, TOOLS_SOURCE);
    assert!(
        codes(&recv_publish(&client).diagnostics).contains(&"OWT102".to_string()),
        "doc A should initially report OWT102"
    );
    open_document(&client, &uri_b, TOOLS_SOURCE);
    assert!(
        codes(&recv_publish(&client).diagnostics).contains(&"OWT102".to_string()),
        "doc B should initially report OWT102"
    );

    // Invoke the disable-rule command for OWT102.
    let command_id = RequestId::from(10);
    send_request(
        &client,
        command_id.clone(),
        "workspace/executeCommand",
        ExecuteCommandParams {
            command: "owui-lint.disableRule".to_string(),
            arguments: vec![serde_json::Value::String("OWT102".to_string())],
            work_done_progress_params: Default::default(),
        },
    );

    // The server republishes diagnostics for both open documents before replying.
    let mut refreshed: HashMap<Url, Vec<String>> = HashMap::new();
    for _ in 0..2 {
        let params = recv_publish(&client);
        refreshed.insert(params.uri, codes(&params.diagnostics));
    }
    recv_response(&client, &command_id);

    for uri in [&uri_a, &uri_b] {
        let codes = refreshed.get(uri).expect("refreshed diagnostics for doc");
        assert!(
            !codes.contains(&"OWT102".to_string()),
            "OWT102 should be silenced for {uri} after disable, got {codes:?}"
        );
        // OWT101 is unaffected, proving we re-linted rather than cleared.
        assert!(
            codes.contains(&"OWT101".to_string()),
            "OWT101 should remain for {uri}, got {codes:?}"
        );
    }

    // The override was persisted to the workspace config file.
    let config =
        std::fs::read_to_string(workspace.path().join("config.yml")).expect("config.yml written");
    assert!(config.contains("OWT102: off"), "config: {config:?}");

    shutdown(&client, server, 11);
}

/// A `didChange` notification (FULL sync) re-lints the new buffer text.
#[test]
fn did_change_relints_updated_buffer() {
    let (client, server) = spawn_server();
    handshake(&client, None);

    let uri = Url::parse("file:///tmp/owui_change.py").expect("uri");
    open_document(&client, &uri, TOOLS_SOURCE);
    assert!(
        codes(&recv_publish(&client).diagnostics).contains(&"OWT102".to_string()),
        "non-async finding expected before the edit"
    );

    // Replace the whole document with an async, documented version.
    send_notification(
        &client,
        "textDocument/didChange",
        DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: TOOLS_SOURCE_FIXED.to_string(),
            }],
        },
    );

    let after = codes(&recv_publish(&client).diagnostics);
    assert!(
        !after.contains(&"OWT102".to_string()),
        "OWT102 should be gone after the async edit, got {after:?}"
    );
    assert!(
        !after.contains(&"OWT101".to_string()),
        "OWT101 should be gone once a docstring exists, got {after:?}"
    );

    shutdown(&client, server, 3);
}

/// Closing a document clears its diagnostics (an empty publish for the URI).
#[test]
fn did_close_clears_diagnostics() {
    let (client, server) = spawn_server();
    handshake(&client, None);

    let uri = Url::parse("file:///tmp/owui_close.py").expect("uri");
    open_document(&client, &uri, TOOLS_SOURCE);
    assert!(
        !recv_publish(&client).diagnostics.is_empty(),
        "open document should report findings"
    );

    send_notification(
        &client,
        "textDocument/didClose",
        DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        },
    );

    let params = recv_publish(&client);
    assert_eq!(params.uri, uri);
    assert!(
        params.diagnostics.is_empty(),
        "closing a document should clear its diagnostics, got {:?}",
        params.diagnostics
    );

    shutdown(&client, server, 3);
}

fn diagnostic_code(diagnostic: &Diagnostic) -> Option<String> {
    match &diagnostic.code {
        Some(NumberOrString::String(code)) => Some(code.clone()),
        _ => None,
    }
}

/// Collect the rule codes from a set of diagnostics.
fn codes(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics.iter().filter_map(diagnostic_code).collect()
}

/// Spawn the language server on an in-memory connection, returning the client
/// end and the server's join handle.
fn spawn_server() -> (Connection, JoinHandle<()>) {
    let (server_conn, client_conn) = Connection::memory();
    let server = std::thread::spawn(move || {
        owui_lint::server::serve(&server_conn).expect("server should run cleanly");
    });
    (client_conn, server)
}

/// Perform the initialize/initialized handshake, optionally advertising a
/// workspace root (needed for config-writing commands).
fn handshake(client: &Connection, root: Option<&Path>) {
    let init_id = RequestId::from(1);
    #[allow(deprecated)]
    let params = InitializeParams {
        root_uri: root.map(|path| Url::from_file_path(path).expect("file uri for root")),
        ..InitializeParams::default()
    };
    send_request(client, init_id.clone(), "initialize", params);
    recv_response(client, &init_id);
    send_notification(client, "initialized", serde_json::json!({}));
}

/// Open a document and let the server lint it.
fn open_document(client: &Connection, uri: &Url, text: &str) {
    send_notification(
        client,
        "textDocument/didOpen",
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "python".to_string(),
                version: 1,
                text: text.to_string(),
            },
        },
    );
}

/// Send `shutdown`/`exit` and join the server thread.
fn shutdown(client: &Connection, server: JoinHandle<()>, id: i32) {
    let shutdown_id = RequestId::from(id);
    send_request(
        client,
        shutdown_id.clone(),
        "shutdown",
        serde_json::Value::Null,
    );
    recv_response(client, &shutdown_id);
    send_notification(client, "exit", serde_json::Value::Null);
    server.join().expect("server thread should join");
}

/// Pull the first `TextEdit` a code action applies to `uri`.
fn first_text_edit(action: &CodeAction, uri: &Url) -> lsp_types::TextEdit {
    action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(uri))
        .and_then(|edits| edits.first())
        .cloned()
        .expect("code action should carry a text edit for the document")
}

fn send_request<P: serde::Serialize>(client: &Connection, id: RequestId, method: &str, params: P) {
    client
        .sender
        .send(Message::Request(Request {
            id,
            method: method.to_string(),
            params: serde_json::to_value(params).expect("serialize request params"),
        }))
        .expect("send request");
}

fn send_notification<P: serde::Serialize>(client: &Connection, method: &str, params: P) {
    client
        .sender
        .send(Message::Notification(Notification {
            method: method.to_string(),
            params: serde_json::to_value(params).expect("serialize notification params"),
        }))
        .expect("send notification");
}

fn recv_response(client: &Connection, expected: &RequestId) -> serde_json::Value {
    loop {
        match recv(client) {
            Message::Response(response) => {
                // A response must match the awaited id. Tolerating a mismatch
                // would let an out-of-order/wrong-id regression hide behind the
                // recv timeout instead of failing clearly here.
                assert_eq!(
                    &response.id, expected,
                    "unexpected response id (awaited {expected:?}): {response:?}"
                );
                assert!(response.error.is_none(), "unexpected error: {response:?}");
                return response.result.unwrap_or(serde_json::Value::Null);
            }
            // Ignore unrelated notifications/requests (e.g. diagnostics).
            Message::Notification(_) | Message::Request(_) => continue,
        }
    }
}

fn recv_publish_diagnostics(client: &Connection) -> Vec<Diagnostic> {
    recv_publish(client).diagnostics
}

/// Wait for the next `publishDiagnostics` notification and return its full
/// params (URI + diagnostics), so callers can attribute diagnostics to a
/// specific document when several are open.
fn recv_publish(client: &Connection) -> PublishDiagnosticsParams {
    loop {
        if let Message::Notification(notification) = recv(client)
            && notification.method == "textDocument/publishDiagnostics"
        {
            return serde_json::from_value(notification.params).expect("diagnostics params");
        }
    }
}

fn recv(client: &Connection) -> Message {
    client
        .receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("server should respond within timeout")
}
