use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

impl Display for Severity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeverityOverride {
    Error,
    Warning,
    Off,
}

impl SeverityOverride {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "error" => Some(Self::Error),
            "warning" => Some(Self::Warning),
            "off" => Some(Self::Off),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxErrorInfo {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionInfo {
    pub name: String,
    pub line: usize,
    pub column: usize,
    pub args: Vec<String>,
    /// Parameter names lacking a type annotation, excluding `self`/`cls`, varargs, and
    /// Open WebUI reserved dunder args. Drives OWT103.
    pub untyped_args: Vec<String>,
    pub decorators: Vec<String>,
    pub is_async: bool,
    pub has_docstring: bool,
    pub returns_annotation: bool,
    pub returns_body: bool,
}

/// Where a detected call site executes, ordered by how early it runs relative to a
/// plugin's lifecycle. Open WebUI `exec()`s the module body and then immediately
/// instantiates the entry class (`module.Tools()`/`Pipe()`/`Filter()`/`Action()` in
/// `plugin.py`), so both `ModuleLevel` and `InitBody` run at *import time* — with no
/// tool call and no user consent. `MethodBody` runs only when the method is invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallScope {
    /// Top-level statements and class-body / Pydantic field-default expressions. Runs
    /// during `exec(content, module.__dict__)`.
    ModuleLevel,
    /// Inside an entry class `__init__`. Runs at import time transitively because OWUI
    /// constructs the entry object right after `exec`.
    InitBody,
    /// Any other method body. Runs only when the method is called.
    MethodBody,
}

impl CallScope {
    /// Both module-level and entry-class `__init__` code execute when Open WebUI loads
    /// the plugin, before any tool call.
    pub fn is_import_time(self) -> bool {
        matches!(self, Self::ModuleLevel | Self::InitBody)
    }
}

/// A call expression detected by the structure-only scanner, tagged with the scope it
/// sits in. `callee` is the dotted reference immediately before the `(` (for example
/// `subprocess.run`, `eval`, `requests.get`). This is a textual detection: no argument
/// or data-flow analysis is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSite {
    pub callee: String,
    pub line: usize,
    pub column: usize,
    pub scope: CallScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValveFieldInfo {
    pub name: String,
    pub line: usize,
    pub has_password_type: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedClassInfo {
    pub name: String,
    pub bases: Vec<String>,
    pub line: usize,
    pub fields: Vec<ValveFieldInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassInfo {
    pub name: String,
    pub line: usize,
    pub column: usize,
    pub bases: Vec<String>,
    pub methods: Vec<FunctionInfo>,
    pub inner_classes: Vec<NestedClassInfo>,
    pub init_assignments: BTreeSet<String>,
    pub has_docstring: bool,
}

impl ClassInfo {
    pub fn method(&self, name: &str) -> Option<&FunctionInfo> {
        self.methods.iter().find(|method| method.name == name)
    }

    pub fn inner_class(&self, name: &str) -> Option<&NestedClassInfo> {
        self.inner_classes
            .iter()
            .find(|class_info| class_info.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInfo {
    pub path: PathBuf,
    pub syntax_ok: bool,
    pub syntax_error: Option<SyntaxErrorInfo>,
    pub module_docstring: Option<String>,
    pub module_docstring_line: Option<usize>,
    pub imports: Vec<String>,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    /// Call sites detected anywhere in the module, tagged with execution scope. Drives
    /// the scope-aware security (`OWSEC`) rules.
    pub call_sites: Vec<CallSite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub rule_id: &'static str,
    pub severity: Severity,
    pub message: String,
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LintSummary {
    pub files_scanned: usize,
    pub errors: usize,
    pub warnings: usize,
}
