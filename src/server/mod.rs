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
    CodeActionRequest, Completion, ExecuteCommand, HoverRequest, Request as _,
};
use lsp_types::{
    CodeActionProviderCapability, CompletionOptions, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, ExecuteCommandOptions,
    ExecuteCommandParams, HoverProviderCapability, InitializeParams, MessageType,
    PublishDiagnosticsParams, ServerCapabilities, ShowMessageParams, TextDocumentSyncCapability,
    TextDocumentSyncKind, Uri,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::server::code_actions::{DISABLE_RULE_COMMAND, disable_rule_in_config};
use crate::server::diagnostics::issues_to_diagnostics;
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
    let capabilities = serde_json::to_value(server_capabilities())
        .context("failed to serialize server capabilities")?;
    let init_params = connection
        .initialize(capabilities)
        .context("LSP initialize handshake failed")?;
    let init_params: InitializeParams =
        serde_json::from_value(init_params).context("invalid InitializeParams")?;

    let mut state = ServerState::new(workspace_root(&init_params));
    main_loop(connection, &mut state)
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

    // Refresh diagnostics for every open document under the new config.
    for uri in state.open_uris() {
        publish_diagnostics(connection, state, &uri)?;
    }
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
