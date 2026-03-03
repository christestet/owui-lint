use std::path::{Path, PathBuf};

use crate::models::{Issue, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleDoc {
    pub id: &'static str,
    pub default_severity: Severity,
    pub title: &'static str,
    pub summary: &'static str,
    pub remediation: &'static str,
    pub help_url: &'static str,
}

const PLUGIN_OVERVIEW: &str = "https://docs.openwebui.com/features/plugin/";
const TOOLS_DOC: &str = "https://docs.openwebui.com/features/plugin/tools/";
const PIPE_DOC: &str = "https://docs.openwebui.com/features/plugin/functions/pipe/";
const FILTER_DOC: &str = "https://docs.openwebui.com/features/plugin/functions/filter/";
const ACTION_DOC: &str = "https://docs.openwebui.com/features/plugin/functions/action/";
const VALVES_DOC: &str = "https://docs.openwebui.com/features/plugin/tools/development/";
const PIPELINES_DOC: &str = "https://docs.openwebui.com/pipelines/";

pub const OWUI001: &str = "OWUI001";
pub const OWUI010: &str = "OWUI010";
pub const OWUI011: &str = "OWUI011";
pub const OWUI020: &str = "OWUI020";
pub const OWUI021: &str = "OWUI021";
pub const OWUI022: &str = "OWUI022";
pub const OWT100: &str = "OWT100";
pub const OWT101: &str = "OWT101";
pub const OWT102: &str = "OWT102";
pub const OWP200: &str = "OWP200";
pub const OWP201: &str = "OWP201";
pub const OWP202: &str = "OWP202";
pub const OWF300: &str = "OWF300";
pub const OWF301: &str = "OWF301";
pub const OWA400: &str = "OWA400";
pub const OWA401: &str = "OWA401";
pub const OWPL500: &str = "OWPL500";
pub const OWPL501: &str = "OWPL501";
pub const OWUI023: &str = "OWUI023";
pub const OWUI030: &str = "OWUI030";
pub const OWUI031: &str = "OWUI031";
pub const OWUI032: &str = "OWUI032";

const RULES: &[RuleDoc] = &[
    RuleDoc {
        id: OWUI001,
        default_severity: Severity::Error,
        title: "Python syntax error",
        summary: "The file cannot be parsed as valid Python.",
        remediation: "Fix syntax errors first; run `python -m py_compile <file.py>` to confirm.",
        help_url: "https://docs.python.org/3/reference/index.html",
    },
    RuleDoc {
        id: OWUI010,
        default_severity: Severity::Warning,
        title: "No extension class detected",
        summary: "The file looks like an extension, but no Tools/Pipe/Filter/Action/Pipeline class was found.",
        remediation: "Define exactly one top-level extension class with a supported name.",
        help_url: PLUGIN_OVERVIEW,
    },
    RuleDoc {
        id: OWUI011,
        default_severity: Severity::Error,
        title: "Mixed extension types",
        summary: "A file contains more than one extension type, which Open WebUI does not support.",
        remediation: "Keep one extension class per file and split other types into separate files.",
        help_url: PLUGIN_OVERVIEW,
    },
    RuleDoc {
        id: OWUI020,
        default_severity: Severity::Warning,
        title: "Missing Valves class",
        summary: "Extensions should provide a nested `Valves` class for runtime configuration.",
        remediation: "Add `class Valves(BaseModel): ...` inside the extension class.",
        help_url: VALVES_DOC,
    },
    RuleDoc {
        id: OWUI021,
        default_severity: Severity::Warning,
        title: "Valves should inherit BaseModel",
        summary: "Valves configuration should inherit from `pydantic.BaseModel`.",
        remediation: "Change `class Valves:` to `class Valves(BaseModel):`.",
        help_url: VALVES_DOC,
    },
    RuleDoc {
        id: OWUI022,
        default_severity: Severity::Warning,
        title: "Valves not initialized",
        summary: "The extension does not initialize `self.valves` in `__init__`.",
        remediation: "Set `self.valves = self.Valves()` in `__init__`.",
        help_url: VALVES_DOC,
    },
    RuleDoc {
        id: OWUI023,
        default_severity: Severity::Warning,
        title: "Sensitive valve field not masked",
        summary: "A Valves field name suggests sensitive data (API key, token, password) but does not use the password input type to mask UI display.",
        remediation: "Add `json_schema_extra={\"input\": {\"type\": \"password\"}}` to the Field() definition.",
        help_url: "https://docs.openwebui.com/features/extensibility/plugin/development/valves#input-types",
    },
    RuleDoc {
        id: OWT100,
        default_severity: Severity::Error,
        title: "No public tool methods",
        summary: "Tools extension must expose at least one callable public method.",
        remediation: "Add an async public method (for example `async def search(...)`) to `Tools`.",
        help_url: TOOLS_DOC,
    },
    RuleDoc {
        id: OWT101,
        default_severity: Severity::Warning,
        title: "Tool method missing docstring",
        summary: "Tool methods should include clear docstrings so users understand capabilities.",
        remediation: "Add a descriptive docstring to each public tool method.",
        help_url: TOOLS_DOC,
    },
    RuleDoc {
        id: OWT102,
        default_severity: Severity::Warning,
        title: "Tool method should be async",
        summary: "Tool methods should be async; Open WebUI calls them in an async context and type-hints generate JSON schemas for the model.",
        remediation: "Use `async def method_name(...)` for all public tool methods.",
        help_url: TOOLS_DOC,
    },
    RuleDoc {
        id: OWP200,
        default_severity: Severity::Error,
        title: "Pipe method missing",
        summary: "Pipe extension must define a `pipe` method.",
        remediation: "Add `async def pipe(self, body, ...)` to the `Pipe` class.",
        help_url: PIPE_DOC,
    },
    RuleDoc {
        id: OWP201,
        default_severity: Severity::Warning,
        title: "Pipe has inlet/outlet",
        summary: "Pipe extensions should not define `inlet` or `outlet` methods.",
        remediation: "Remove `inlet`/`outlet`, or convert this class to a `Filter` extension.",
        help_url: PIPE_DOC,
    },
    RuleDoc {
        id: OWP202,
        default_severity: Severity::Warning,
        title: "Pipe method should be async",
        summary: "Synchronous `pipe` methods reduce compatibility with Open WebUI runtime execution.",
        remediation: "Use `async def pipe(...)` and await I/O operations.",
        help_url: PIPE_DOC,
    },
    RuleDoc {
        id: OWF300,
        default_severity: Severity::Error,
        title: "Filter has no inlet/outlet/stream",
        summary: "Filter extension must implement `inlet`, `outlet`, `stream`, or a combination of them.",
        remediation: "Add at least one of `inlet`, `outlet`, or `stream` methods.",
        help_url: FILTER_DOC,
    },
    RuleDoc {
        id: OWF301,
        default_severity: Severity::Warning,
        title: "inlet should return body",
        summary: "`Filter.inlet` should return the transformed request body.",
        remediation: "Return `body` (or the modified body) from `inlet`.",
        help_url: FILTER_DOC,
    },
    RuleDoc {
        id: OWA400,
        default_severity: Severity::Error,
        title: "Action method missing",
        summary: "Action extension must define an `action` method.",
        remediation: "Add `async def action(self, body, ...)` to the class.",
        help_url: ACTION_DOC,
    },
    RuleDoc {
        id: OWA401,
        default_severity: Severity::Warning,
        title: "Action should be async",
        summary: "Synchronous `action` methods may not behave correctly in async execution contexts.",
        remediation: "Use `async def action(...)` and await I/O operations.",
        help_url: ACTION_DOC,
    },
    RuleDoc {
        id: OWPL500,
        default_severity: Severity::Error,
        title: "Pipeline missing processing hook",
        summary: "Pipeline extension must define `pipe` (pipe type), `pipes` (manifold type), or filter hooks (`inlet`/`outlet`/`stream`).",
        remediation: "Add `pipe` for a standard pipeline, `pipes` for a manifold returning multiple models, or filter hooks for a filter-type pipeline.",
        help_url: PIPELINES_DOC,
    },
    RuleDoc {
        id: OWPL501,
        default_severity: Severity::Warning,
        title: "Pipeline name not assigned",
        summary: "`Pipeline.__init__` should assign `self.name` for clearer labeling.",
        remediation: "Set `self.name = \"...\"` in `__init__`.",
        help_url: PIPELINES_DOC,
    },
    RuleDoc {
        id: OWUI030,
        default_severity: Severity::Warning,
        title: "Missing version in module header",
        summary: "The module docstring header does not include a `version:` field.",
        remediation: "Add `version: 0.1.0` (or your current version) to the module docstring.",
        help_url: PLUGIN_OVERVIEW,
    },
    RuleDoc {
        id: OWUI031,
        default_severity: Severity::Warning,
        title: "Unpinned requirements in module header",
        summary: "One or more packages in `requirements:` lack a pinned version specifier.",
        remediation: "Pin each package, e.g. change `llama-index` to `llama-index==0.1.2`.",
        help_url: PLUGIN_OVERVIEW,
    },
    RuleDoc {
        id: OWUI032,
        default_severity: Severity::Warning,
        title: "Missing title in module header",
        summary: "The module docstring header does not include a `title:` field, which Open WebUI uses as the display name in the UI.",
        remediation: "Add `title: My Extension Name` to the module docstring.",
        help_url: PLUGIN_OVERVIEW,
    },
];

pub fn all_rules() -> &'static [RuleDoc] {
    RULES
}

pub fn rule_doc(rule_id: &str) -> Option<&'static RuleDoc> {
    RULES.iter().find(|rule| rule.id == rule_id)
}

pub fn is_known_rule(rule_id: &str) -> bool {
    rule_doc(rule_id).is_some()
}

pub fn issue(
    rule_id: &'static str,
    path: &Path,
    line: usize,
    column: usize,
    message: impl Into<String>,
) -> Issue {
    let rule = rule_doc(rule_id)
        .unwrap_or_else(|| panic!("Unknown rule ID '{rule_id}'. Add it to src/rules.rs first."));

    Issue {
        rule_id,
        severity: rule.default_severity,
        message: message.into(),
        path: PathBuf::from(path),
        line,
        column,
    }
}
