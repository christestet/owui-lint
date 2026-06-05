//! Language Server Protocol implementation for owui-lint.
//!
//! The server publishes Open WebUI-specific diagnostics for the buffer the user
//! is editing and complements (does not replace) a general Python language
//! server such as Pylance/Pyright. It speaks LSP over stdio via `lsp-server`.

// `lsp_types::Uri` (fluent-uri) carries an internal cache cell, so Clippy's
// `mutable_key_type` lint fires when we key maps by it. Its `Hash`/`Eq` use the
// stable string form (`as_str()`), so using it as a map key is sound.
#![allow(clippy::mutable_key_type)]

mod code_actions;
mod completions;
mod diagnostics;
mod hover;
mod position;
mod state;

use anyhow::{Context, Result};
use lsp_server::{Connection, ErrorCode, Message, RequestId, Response, ResponseError};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics, ShowMessage,
};
use lsp_types::request::{
    CodeActionRequest, Completion, DocumentDiagnosticRequest, ExecuteCommand, HoverRequest,
    Request as _, WorkspaceDiagnosticRefresh,
};
use lsp_types::{
    CodeActionProviderCapability, CompletionOptions, DiagnosticOptions,
    DiagnosticServerCapabilities, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, ExecuteCommandOptions, ExecuteCommandParams,
    FullDocumentDiagnosticReport, HoverProviderCapability, InitializeParams, InitializeResult,
    MessageType, PublishDiagnosticsParams, RelatedFullDocumentDiagnosticReport, ServerCapabilities,
    ServerInfo, ShowMessageParams, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
    WorkDoneProgressOptions,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::server::code_actions::{DISABLE_RULE_COMMAND, disable_rule_in_config};
use crate::server::diagnostics::{DIAGNOSTIC_SOURCE, issues_to_diagnostics};
use crate::server::state::{ServerState, uri_to_file_path};

/// Run the language server over stdio until the client shuts it down.
pub fn run() -> Result<()> {
    let (connection, io_threads) = Connection::stdio();
    serve(&connection)?;
    // Drop the connection (and its sender) before joining so the stdio writer
    // thread can finish; otherwise `join` would block forever.
    drop(connection);
    io_threads.join().context("LSP io threads panicked")?;
    Ok(())
}

/// Perform the initialize handshake and run the main loop on a connection.
/// Decoupled from `Connection::stdio()` so tests can drive the server through an
/// in-memory connection (`Connection::memory()`).
pub fn serve(connection: &Connection) -> Result<()> {
    // Drive the handshake by hand (rather than `Connection::initialize`) so the
    // `InitializeResult` can carry `serverInfo` and so we can read the client's
    // capabilities to decide which optional features to use.
    let (id, params) = connection
        .initialize_start()
        .context("LSP initialize handshake failed")?;
    let init_params: InitializeParams =
        serde_json::from_value(params).context("invalid InitializeParams")?;

    let result = InitializeResult {
        capabilities: server_capabilities(),
        server_info: Some(ServerInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    };
    connection
        .initialize_finish(id, serde_json::to_value(result)?)
        .context("LSP initialize handshake failed")?;

    let mut state = ServerState::new(workspace_root(&init_params));
    state.set_diagnostic_refresh_support(client_supports_diagnostic_refresh(&init_params));
    main_loop(connection, &mut state)
}

/// Whether the client advertised `workspace.diagnostic.refreshSupport`, meaning
/// it can handle a server-sent `workspace/diagnostic/refresh` request.
fn client_supports_diagnostic_refresh(params: &InitializeParams) -> bool {
    params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.diagnostic.as_ref())
        .and_then(|diagnostic| diagnostic.refresh_support)
        .unwrap_or(false)
}

fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_string()]),
            ..CompletionOptions::default()
        }),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec![DISABLE_RULE_COMMAND.to_string()],
            ..ExecuteCommandOptions::default()
        }),
        // Pull-model diagnostics (LSP 3.17). Each file is linted in isolation, so
        // there are no inter-file dependencies and we don't offer workspace-wide
        // pull. Push diagnostics (publishDiagnostics) remain for clients that use
        // them; clients that support pull will prefer it and ignore the push.
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            identifier: Some(DIAGNOSTIC_SOURCE.to_string()),
            inter_file_dependencies: false,
            workspace_diagnostics: false,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        ..ServerCapabilities::default()
    }
}

fn workspace_root(params: &InitializeParams) -> Option<std::path::PathBuf> {
    if let Some(folders) = &params.workspace_folders
        && let Some(first) = folders.first()
        && let Some(path) = uri_to_file_path(&first.uri)
    {
        return Some(path);
    }
    #[allow(deprecated)]
    params.root_uri.as_ref().and_then(uri_to_file_path)
}

fn main_loop(connection: &Connection, state: &mut ServerState) -> Result<()> {
    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection
                    .handle_shutdown(&request)
                    .context("shutdown handshake failed")?
                {
                    return Ok(());
                }
                // A failure handling one request (e.g. malformed params) must not
                // tear down the whole session: log it and keep serving. The
                // client has already received an error response where relevant.
                if let Err(err) = dispatch_request(connection, state, request) {
                    eprintln!("owui-lint: error handling request: {err:#}");
                }
            }
            Message::Notification(notification) => {
                if let Err(err) = dispatch_notification(connection, state, notification) {
                    eprintln!("owui-lint: error handling notification: {err:#}");
                }
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn dispatch_request(
    connection: &Connection,
    state: &mut ServerState,
    request: lsp_server::Request,
) -> Result<()> {
    let id = request.id.clone();
    match request.method.as_str() {
        HoverRequest::METHOD => {
            let Some(params) = parse_params(connection, &id, request.params)? else {
                return Ok(());
            };
            let result = hover::handle_hover(state, &params);
            respond(connection, id, &result)?;
        }
        CodeActionRequest::METHOD => {
            let Some(params) = parse_params(connection, &id, request.params)? else {
                return Ok(());
            };
            let result = code_actions::handle_code_action(state, &params);
            respond(connection, id, &result)?;
        }
        Completion::METHOD => {
            let Some(params) = parse_params(connection, &id, request.params)? else {
                return Ok(());
            };
            let result = completions::handle_completion(state, &params);
            respond(connection, id, &result)?;
        }
        DocumentDiagnosticRequest::METHOD => {
            let Some(params) =
                parse_params::<DocumentDiagnosticParams>(connection, &id, request.params)?
            else {
                return Ok(());
            };
            let result = document_diagnostic_report(state, &params.text_document.uri);
            respond(connection, id, &result)?;
        }
        ExecuteCommand::METHOD => {
            let Some(params) =
                parse_params::<ExecuteCommandParams>(connection, &id, request.params)?
            else {
                return Ok(());
            };
            handle_execute_command(connection, state, &params)?;
            respond(connection, id, &serde_json::Value::Null)?;
        }
        method => {
            // Unsupported method: reply with a proper JSON-RPC error so the
            // client is not left waiting (and learns we don't handle it).
            respond_error(
                connection,
                id,
                ErrorCode::MethodNotFound,
                format!("unsupported method: {method}"),
            )?;
        }
    }
    Ok(())
}

fn dispatch_notification(
    connection: &Connection,
    state: &mut ServerState,
    notification: lsp_server::Notification,
) -> Result<()> {
    match notification.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(notification.params)?;
            let document = params.text_document;
            state.upsert(document.uri.clone(), document.text, document.version);
            publish_diagnostics(connection, state, &document.uri)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(notification.params)?;
            let uri = params.text_document.uri;
            let version = params.text_document.version;
            // FULL sync: the last change carries the entire document text. An
            // empty change list still advances the version, so record the new
            // version even when there is no text to re-lint.
            if let Some(change) = params.content_changes.into_iter().last() {
                state.upsert(uri.clone(), change.text, version);
                publish_diagnostics(connection, state, &uri)?;
            } else {
                state.set_version(&uri, version);
            }
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params)?;
            state.remove(&params.text_document.uri);
            // Clear diagnostics for the closed file.
            publish_for(connection, &params.text_document.uri, Vec::new(), None)?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_execute_command(
    connection: &Connection,
    state: &mut ServerState,
    params: &ExecuteCommandParams,
) -> Result<()> {
    if params.command != DISABLE_RULE_COMMAND {
        return Ok(());
    }
    let Some(rule_id) = params.arguments.first().and_then(|value| value.as_str()) else {
        return Ok(());
    };
    let Some(root) = state.root() else {
        return Ok(());
    };

    if let Err(err) = disable_rule_in_config(root, rule_id) {
        let message = format!("owui-lint: failed to update config to disable {rule_id}: {err}");
        eprintln!("{message}");
        show_message(connection, MessageType::ERROR, message)?;
        return Ok(());
    }
    state.reload_config();

    // Refresh diagnostics for every open document under the new config (push
    // model). Pull-model clients get a single refresh request instead.
    for uri in state.open_uris() {
        publish_diagnostics(connection, state, &uri)?;
    }
    if state.diagnostic_refresh_support() {
        request_diagnostic_refresh(connection, state)?;
    }
    Ok(())
}

/// Build a full diagnostic report for a pull (`textDocument/diagnostic`) request.
/// This is the pull-shaped view of the same cached issues that `publish_diagnostics`
/// pushes. Unknown (never-opened) documents report an empty set.
fn document_diagnostic_report(state: &ServerState, uri: &Uri) -> DocumentDiagnosticReportResult {
    let items = state
        .document(uri)
        .map(|document| issues_to_diagnostics(&document.issues, &document.text))
        .unwrap_or_default();
    DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
        RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id: None,
                items,
            },
        },
    ))
}

/// Ask the client to re-pull all diagnostics. Sent after a project-wide change
/// (e.g. disabling a rule) so pull-model clients see the new results. The client
/// replies to this request; the main loop ignores that response.
fn request_diagnostic_refresh(connection: &Connection, state: &mut ServerState) -> Result<()> {
    let request = lsp_server::Request {
        id: RequestId::from(state.next_request_id()),
        method: WorkspaceDiagnosticRefresh::METHOD.to_string(),
        params: serde_json::Value::Null,
    };
    connection
        .sender
        .send(Message::Request(request))
        .context("failed to send diagnostic refresh request")?;
    Ok(())
}

fn publish_diagnostics(connection: &Connection, state: &ServerState, uri: &Uri) -> Result<()> {
    let Some(document) = state.document(uri) else {
        return Ok(());
    };
    let diagnostics = issues_to_diagnostics(&document.issues, &document.text);
    publish_for(connection, uri, diagnostics, Some(document.version))
}

fn publish_for(
    connection: &Connection,
    uri: &Uri,
    diagnostics: Vec<lsp_types::Diagnostic>,
    version: Option<i32>,
) -> Result<()> {
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version,
    };
    let notification = lsp_server::Notification {
        method: PublishDiagnostics::METHOD.to_string(),
        params: serde_json::to_value(params)?,
    };
    connection
        .sender
        .send(Message::Notification(notification))
        .context("failed to send diagnostics")?;
    Ok(())
}

/// Deserialize request params, or send an `InvalidParams` error response and
/// return `None` so the caller can stop handling this request without aborting
/// the session.
fn parse_params<T: DeserializeOwned>(
    connection: &Connection,
    id: &RequestId,
    params: serde_json::Value,
) -> Result<Option<T>> {
    match serde_json::from_value(params) {
        Ok(value) => Ok(Some(value)),
        Err(err) => {
            respond_error(
                connection,
                id.clone(),
                ErrorCode::InvalidParams,
                format!("invalid params: {err}"),
            )?;
            Ok(None)
        }
    }
}

fn respond<T: Serialize>(connection: &Connection, id: RequestId, result: &T) -> Result<()> {
    let response = Response {
        id,
        result: Some(serde_json::to_value(result)?),
        error: None,
    };
    connection
        .sender
        .send(Message::Response(response))
        .context("failed to send response")?;
    Ok(())
}

fn respond_error(
    connection: &Connection,
    id: RequestId,
    code: ErrorCode,
    message: String,
) -> Result<()> {
    let response = Response {
        id,
        result: None,
        error: Some(ResponseError {
            code: code as i32,
            message,
            data: None,
        }),
    };
    connection
        .sender
        .send(Message::Response(response))
        .context("failed to send error response")?;
    Ok(())
}

/// Send a `window/showMessage` notification so the user sees server-side
/// failures (e.g. a config write that did not go through) in their editor.
fn show_message(connection: &Connection, typ: MessageType, message: String) -> Result<()> {
    let params = ShowMessageParams { typ, message };
    let notification = lsp_server::Notification {
        method: ShowMessage::METHOD.to_string(),
        params: serde_json::to_value(params)?,
    };
    connection
        .sender
        .send(Message::Notification(notification))
        .context("failed to send showMessage")?;
    Ok(())
}
