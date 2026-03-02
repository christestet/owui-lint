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
    pub decorators: Vec<String>,
    pub is_async: bool,
    pub has_docstring: bool,
    pub returns_annotation: bool,
    pub returns_body: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedClassInfo {
    pub name: String,
    pub bases: Vec<String>,
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
