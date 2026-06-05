use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::Uri;

use crate::config::{Config, load_config_in_dir};
use crate::linter::lint_source;
use crate::models::Issue;

/// An open document tracked by the language server. The server keeps the full
/// text in memory (sync kind FULL) so it can lint unsaved buffers. The lint
/// `issues` are computed once per `(text, config)` and cached here, so every
/// feature (diagnostics, hover, ...) reads a single consistent result instead
/// of re-linting per request.
#[derive(Debug, Clone)]
pub struct Document {
    pub text: String,
    pub version: i32,
    pub issues: Vec<Issue>,
}

/// Mutable server state: open documents plus the workspace configuration.
#[derive(Debug)]
pub struct ServerState {
    documents: HashMap<Uri, Document>,
    root: Option<PathBuf>,
    config: Config,
    /// Whether the client advertised `workspace.diagnostic.refreshSupport`, i.e.
    /// it can handle a server-sent `workspace/diagnostic/refresh` request. Set
    /// from the client capabilities during the initialize handshake.
    diagnostic_refresh_support: bool,
    /// Monotonic id source for requests the server sends to the client (e.g. the
    /// diagnostic refresh request). JSON-RPC ids must be unique per connection.
    next_request_id: i32,
}

impl ServerState {
    /// Build state for a workspace root, loading its config (or defaults).
    pub fn new(root: Option<PathBuf>) -> Self {
        let config = match &root {
            Some(dir) => load_config_in_dir(dir).unwrap_or_default(),
            None => Config::default(),
        };
        Self {
            documents: HashMap::new(),
            root,
            config,
            diagnostic_refresh_support: false,
            next_request_id: 1,
        }
    }

    /// Record whether the client supports `workspace/diagnostic/refresh`.
    pub fn set_diagnostic_refresh_support(&mut self, supported: bool) {
        self.diagnostic_refresh_support = supported;
    }

    /// Whether the client can handle a server-sent diagnostic refresh request.
    pub fn diagnostic_refresh_support(&self) -> bool {
        self.diagnostic_refresh_support
    }

    /// Allocate the next unique id for a server-initiated request.
    pub fn next_request_id(&mut self) -> i32 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }

    /// Read accessor for the active config. Linting now happens inside the state
    /// (see `upsert`), so only tests inspect the config directly.
    #[cfg(test)]
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn document(&self, uri: &Uri) -> Option<&Document> {
        self.documents.get(uri)
    }

    /// All currently open document URIs (used to refresh diagnostics).
    pub fn open_uris(&self) -> Vec<Uri> {
        self.documents.keys().cloned().collect()
    }

    pub fn upsert(&mut self, uri: Uri, text: String, version: i32) {
        let issues = lint_source(&uri_to_path(&uri), &text, &self.config);
        self.documents.insert(
            uri,
            Document {
                text,
                version,
                issues,
            },
        );
    }

    /// Record a new document version without changing its text or re-linting.
    /// Used for `didChange` notifications that carry no content (FULL sync still
    /// advances the version even when the text is unchanged).
    pub fn set_version(&mut self, uri: &Uri, version: i32) {
        if let Some(document) = self.documents.get_mut(uri) {
            document.version = version;
        }
    }

    pub fn remove(&mut self, uri: &Uri) {
        self.documents.remove(uri);
    }

    /// Reload configuration from the workspace root, e.g. after a quick-fix
    /// wrote a rule override to the config file, then re-lint every open
    /// document so cached issues reflect the new config.
    pub fn reload_config(&mut self) {
        if let Some(dir) = &self.root {
            self.config = load_config_in_dir(dir).unwrap_or_default();
        }
        let Self {
            documents, config, ..
        } = self;
        for (uri, document) in documents.iter_mut() {
            document.issues = lint_source(&uri_to_path(uri), &document.text, config);
        }
    }
}

/// Convert a `file://` URI to a real filesystem path, or `None` for non-file
/// URIs. `lsp_types::Uri` (fluent-uri) has no path conversion, so we go through
/// the `url` crate which handles percent-decoding and platform-specific paths.
pub fn uri_to_file_path(uri: &Uri) -> Option<PathBuf> {
    url::Url::parse(uri.as_str())
        .ok()
        .and_then(|url| url.to_file_path().ok())
}

/// Convert a `file://` URI to a filesystem path. Diagnostics need a path so the
/// analyzer can resolve the extension type; for non-file URIs we fall back to a
/// best-effort path derived from the URI itself.
pub fn uri_to_path(uri: &Uri) -> PathBuf {
    uri_to_file_path(uri).unwrap_or_else(|| {
        url::Url::parse(uri.as_str())
            .map(|url| PathBuf::from(url.path()))
            .unwrap_or_else(|_| PathBuf::from(uri.as_str()))
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn uri(s: &str) -> Uri {
        Uri::from_str(s).expect("valid uri")
    }

    #[test]
    fn uri_to_path_handles_file_uri() {
        let path = uri_to_path(&uri("file:///tmp/owui/tools.py"));
        assert_eq!(path, PathBuf::from("/tmp/owui/tools.py"));
        // The `.py` extension survives so the analyzer can detect the file type.
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("py"));
    }

    #[test]
    fn uri_to_path_falls_back_for_non_file_uri() {
        // Non-`file://` scheme (e.g. an unsaved/virtual buffer): best-effort path
        // from the URI path component, still ending in `.py`.
        let path = uri_to_path(&uri("untitled:/Untitled-1.py"));
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("py"));
    }

    #[test]
    fn upsert_and_document_roundtrip() {
        let mut state = ServerState::new(None);
        let doc_uri = uri("file:///tmp/a.py");
        state.upsert(doc_uri.clone(), "print(1)".to_string(), 7);

        let doc = state.document(&doc_uri).expect("document stored");
        assert_eq!(doc.text, "print(1)");
        assert_eq!(doc.version, 7);

        // upsert replaces (last write wins for FULL sync).
        state.upsert(doc_uri.clone(), "print(2)".to_string(), 8);
        assert_eq!(state.document(&doc_uri).expect("doc").text, "print(2)");
        assert_eq!(state.document(&doc_uri).expect("doc").version, 8);
    }

    #[test]
    fn set_version_bumps_version_without_changing_text() {
        let mut state = ServerState::new(None);
        let doc_uri = uri("file:///tmp/a.py");
        state.upsert(doc_uri.clone(), "print(1)".to_string(), 3);

        // An empty `didChange` advances the version but leaves text/issues alone.
        state.set_version(&doc_uri, 4);
        let doc = state.document(&doc_uri).expect("doc");
        assert_eq!(doc.version, 4);
        assert_eq!(doc.text, "print(1)");

        // A no-op on an unknown document is harmless.
        state.set_version(&uri("file:///tmp/missing.py"), 9);
    }

    #[test]
    fn open_uris_and_remove() {
        let mut state = ServerState::new(None);
        let a = uri("file:///tmp/a.py");
        let b = uri("file:///tmp/b.py");
        state.upsert(a.clone(), String::new(), 1);
        state.upsert(b.clone(), String::new(), 1);

        let mut open = state.open_uris();
        open.sort();
        assert_eq!(open, vec![a.clone(), b.clone()]);

        state.remove(&a);
        assert!(state.document(&a).is_none());
        assert_eq!(state.open_uris(), vec![b]);
    }

    #[test]
    fn new_without_root_uses_default_config() {
        let state = ServerState::new(None);
        assert_eq!(state.config(), &Config::default());
        assert!(state.root().is_none());
    }

    #[test]
    fn new_loads_config_from_root_and_reloads() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("config.yml"), "rules:\n  OWT102: off\n")
            .expect("seed config");

        let mut state = ServerState::new(Some(dir.path().to_path_buf()));
        assert!(state.config().rule_overrides.contains_key("OWT102"));

        // Simulate a quick-fix appending another override, then reloading.
        std::fs::write(
            dir.path().join("config.yml"),
            "rules:\n  OWT102: off\n  OWT101: off\n",
        )
        .expect("update config");
        state.reload_config();
        assert!(state.config().rule_overrides.contains_key("OWT101"));
    }
}
