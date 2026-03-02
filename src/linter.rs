use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use walkdir::WalkDir;

use crate::analysis::analyze_file;
use crate::config::Config;
use crate::glob::glob_match;
use crate::models::{ClassInfo, Issue, LintSummary, ModuleInfo, Severity, SeverityOverride};
use crate::rules::{
    OWA400, OWA401, OWF300, OWF301, OWF302, OWP200, OWP201, OWP202, OWPL500, OWPL501, OWT100,
    OWT101, OWUI001, OWUI010, OWUI011, OWUI020, OWUI021, OWUI022, OWUI030, OWUI031, issue,
};

const EXTENSION_CLASSES: [(&str, &str); 5] = [
    ("Tools", "tools"),
    ("Pipe", "pipe"),
    ("Filter", "filter"),
    ("Action", "action"),
    ("Pipeline", "pipeline"),
];

pub fn discover_python_files(
    targets: &[PathBuf],
    include_patterns: &[String],
    exclude_patterns: &[String],
) -> Result<Vec<PathBuf>> {
    if targets.is_empty() {
        return Err(anyhow!(
            "At least one file or directory target is required."
        ));
    }

    let cwd = std::env::current_dir()?;
    let mut resolved: BTreeSet<PathBuf> = BTreeSet::new();

    for target in targets {
        if !target.exists() {
            return Err(anyhow!("Target not found: {}", target.display()));
        }

        if target.is_file() {
            if target.extension().and_then(|ext| ext.to_str()) == Some("py") {
                resolved.insert(canonical(target)?);
            }
            continue;
        }

        for entry in WalkDir::new(target)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let path = entry.into_path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("py") {
                resolved.insert(canonical(&path)?);
            }
        }
    }

    let files = resolved
        .into_iter()
        .filter(|path| included(path, &cwd, include_patterns, exclude_patterns))
        .collect();

    Ok(files)
}

pub fn lint_files(files: &[PathBuf], config: &Config) -> (Vec<Issue>, LintSummary) {
    let mut issues: Vec<Issue> = Vec::new();

    for file_path in files {
        let module_info = analyze_file(file_path);
        issues.extend(lint_module(&module_info));
    }

    let mut filtered = apply_rule_overrides(issues, config);
    filtered.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column))
            .then_with(|| left.rule_id.cmp(right.rule_id))
    });

    let errors = filtered
        .iter()
        .filter(|issue| issue.severity == Severity::Error)
        .count();
    let warnings = filtered
        .iter()
        .filter(|issue| issue.severity == Severity::Warning)
        .count();

    (
        filtered,
        LintSummary {
            files_scanned: files.len(),
            errors,
            warnings,
        },
    )
}

fn apply_rule_overrides(mut issues: Vec<Issue>, config: &Config) -> Vec<Issue> {
    let mut filtered = Vec::with_capacity(issues.len());
    for mut issue in issues.drain(..) {
        match config.rule_overrides.get(issue.rule_id) {
            Some(SeverityOverride::Off) => continue,
            Some(SeverityOverride::Error) => issue.severity = Severity::Error,
            Some(SeverityOverride::Warning) => issue.severity = Severity::Warning,
            None => {}
        }
        filtered.push(issue);
    }
    filtered
}

fn lint_module(module: &ModuleInfo) -> Vec<Issue> {
    let mut issues = Vec::new();

    if !module.syntax_ok {
        let (line, column, message) = match &module.syntax_error {
            Some(error) => (error.line, error.column, error.message.as_str()),
            None => (1, 1, "Syntax error"),
        };
        issues.push(issue(
            OWUI001,
            &module.path,
            line,
            column,
            format!("Python syntax error: {message}"),
        ));
        return issues;
    }

    issues.extend(lint_module_header(&module.path, module));

    let extension_classes: Vec<&ClassInfo> = module
        .classes
        .iter()
        .filter(|class_info| extension_type(&class_info.name).is_some())
        .collect();

    if extension_classes.is_empty() {
        if looks_like_openwebui_extension(module) {
            issues.push(issue(
                OWUI010,
                &module.path,
                1,
                1,
                "No Open WebUI extension class found (Tools, Pipe, Filter, Action, Pipeline).",
            ));
        }
        return issues;
    }

    if extension_classes.len() > 1 {
        let class_names = extension_classes
            .iter()
            .map(|class_info| class_info.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        issues.push(issue(
            OWUI011,
            &module.path,
            extension_classes[0].line,
            extension_classes[0].column,
            format!("Mixed extension types in one file are not supported: {class_names}."),
        ));
    }

    for class_info in extension_classes {
        issues.extend(lint_extension_class(&module.path, class_info));
    }

    issues
}

fn extension_type(class_name: &str) -> Option<&'static str> {
    EXTENSION_CLASSES
        .iter()
        .find_map(|(name, class_type)| (*name == class_name).then_some(*class_type))
}

fn looks_like_openwebui_extension(module: &ModuleInfo) -> bool {
    let suspicious_names = [
        "tool",
        "tools",
        "pipe",
        "pipes",
        "filter",
        "filters",
        "action",
        "actions",
        "pipeline",
        "pipelines",
    ];
    let suspicious_methods = [
        "pipe",
        "pipes",
        "inlet",
        "outlet",
        "stream",
        "action",
        "on_startup",
        "on_shutdown",
        "on_valves_updated",
    ];

    module.classes.iter().any(|class_info| {
        suspicious_names.contains(&class_info.name.to_ascii_lowercase().as_str())
            || class_info
                .methods
                .iter()
                .any(|method| suspicious_methods.contains(&method.name.as_str()))
    })
}

fn lint_extension_class(path: &Path, class_info: &ClassInfo) -> Vec<Issue> {
    let mut issues = lint_common(path, class_info);

    match extension_type(&class_info.name) {
        Some("tools") => issues.extend(lint_tools(path, class_info)),
        Some("pipe") => issues.extend(lint_pipe(path, class_info)),
        Some("filter") => issues.extend(lint_filter(path, class_info)),
        Some("action") => issues.extend(lint_action(path, class_info)),
        Some("pipeline") => issues.extend(lint_pipeline(path, class_info)),
        _ => {}
    }

    issues
}

fn lint_common(path: &Path, class_info: &ClassInfo) -> Vec<Issue> {
    let mut issues = Vec::new();

    let Some(valves) = class_info.inner_class("Valves") else {
        issues.push(issue(
            OWUI020,
            path,
            class_info.line,
            class_info.column,
            format!(
                "{} should define an inner Valves class for configuration.",
                class_info.name
            ),
        ));
        return issues;
    };

    let has_base_model = valves
        .bases
        .iter()
        .map(|base| base.split('.').next_back().unwrap_or(base.as_str()))
        .any(|base| base == "BaseModel");

    if !has_base_model {
        issues.push(issue(
            OWUI021,
            path,
            class_info.line,
            class_info.column,
            "Valves should inherit from pydantic.BaseModel.",
        ));
    }

    if !class_info.init_assignments.contains("valves") {
        issues.push(issue(
            OWUI022,
            path,
            class_info.line,
            class_info.column,
            "Initialize valves in __init__ with self.valves = self.Valves().",
        ));
    }

    issues
}

fn lint_tools(path: &Path, class_info: &ClassInfo) -> Vec<Issue> {
    let ignored: BTreeSet<&str> = [
        "__init__",
        "pipes",
        "pipe",
        "inlet",
        "outlet",
        "action",
        "on_startup",
        "on_shutdown",
        "on_valves_updated",
    ]
    .into_iter()
    .collect();

    let tool_methods: Vec<_> = class_info
        .methods
        .iter()
        .filter(|method| !method.name.starts_with('_') && !ignored.contains(method.name.as_str()))
        .collect();

    if tool_methods.is_empty() {
        return vec![issue(
            OWT100,
            path,
            class_info.line,
            class_info.column,
            "Tools class has no public tool methods.",
        )];
    }

    let mut issues = Vec::new();
    for method in tool_methods {
        if !method.has_docstring {
            issues.push(issue(
                OWT101,
                path,
                method.line,
                method.column,
                format!(
                    "Tool method '{}' should include a descriptive docstring.",
                    method.name
                ),
            ));
        }
    }

    issues
}

fn lint_pipe(path: &Path, class_info: &ClassInfo) -> Vec<Issue> {
    let mut issues = Vec::new();
    let inlet_method = class_info.method("inlet");
    let outlet_method = class_info.method("outlet");

    let Some(pipe_method) = class_info.method("pipe") else {
        return vec![issue(
            OWP200,
            path,
            class_info.line,
            class_info.column,
            "Pipe class must define a 'pipe' method.",
        )];
    };

    if inlet_method.is_some() || outlet_method.is_some() {
        issues.push(issue(
            OWP201,
            path,
            class_info.line,
            class_info.column,
            "Pipe classes must not define inlet/outlet methods; use Filter instead.",
        ));
    }

    if !pipe_method.is_async {
        issues.push(issue(
            OWP202,
            path,
            pipe_method.line,
            pipe_method.column,
            "Pipe.pipe should be async for compatibility with Open WebUI execution.",
        ));
    }

    issues
}

fn lint_filter(path: &Path, class_info: &ClassInfo) -> Vec<Issue> {
    let mut issues = Vec::new();
    let inlet = class_info.method("inlet");
    let outlet = class_info.method("outlet");
    let stream = class_info.method("stream");

    if inlet.is_none() && outlet.is_none() && stream.is_none() {
        return vec![issue(
            OWF300,
            path,
            class_info.line,
            class_info.column,
            "Filter must define at least one of inlet/outlet/stream.",
        )];
    }

    if let Some(inlet) = inlet
        && !inlet.returns_body
    {
        issues.push(issue(
            OWF301,
            path,
            inlet.line,
            inlet.column,
            "Filter.inlet should return body.",
        ));
    }

    if let Some(outlet) = outlet
        && !outlet.returns_body
    {
        issues.push(issue(
            OWF302,
            path,
            outlet.line,
            outlet.column,
            "Filter.outlet should return body.",
        ));
    }

    issues
}

fn lint_action(path: &Path, class_info: &ClassInfo) -> Vec<Issue> {
    let mut issues = Vec::new();

    let Some(action) = class_info.method("action") else {
        return vec![issue(
            OWA400,
            path,
            class_info.line,
            class_info.column,
            "Action class must define an 'action' method.",
        )];
    };

    if !action.is_async {
        issues.push(issue(
            OWA401,
            path,
            action.line,
            action.column,
            "Action.action should be async.",
        ));
    }

    issues
}

fn lint_pipeline(path: &Path, class_info: &ClassInfo) -> Vec<Issue> {
    let mut issues = Vec::new();
    let has_pipe = class_info.method("pipe").is_some();
    let has_filter_hook = class_info.method("inlet").is_some()
        || class_info.method("outlet").is_some()
        || class_info.method("stream").is_some();

    if !has_pipe && !has_filter_hook {
        issues.push(issue(
            OWPL500,
            path,
            class_info.line,
            class_info.column,
            "Pipeline class must define 'pipe' or at least one filter hook (inlet/outlet/stream).",
        ));
    }

    if !class_info.init_assignments.contains("name") {
        issues.push(issue(
            OWPL501,
            path,
            class_info.line,
            class_info.column,
            "Pipeline __init__ should set self.name for clearer model labeling.",
        ));
    }

    issues
}

fn has_version_specifier(req: &str) -> bool {
    ["==", ">=", "<=", "!=", "~=", ">", "<", "@"]
        .iter()
        .any(|spec| req.contains(spec))
}

fn lint_module_header(path: &Path, module: &ModuleInfo) -> Vec<Issue> {
    let Some(docstring) = &module.module_docstring else {
        return Vec::new();
    };
    let line = module.module_docstring_line.unwrap_or(1);
    let mut issues = Vec::new();

    let mut has_version = false;
    let mut req_value: Option<String> = None;

    for ds_line in docstring.lines() {
        let t = ds_line.trim();
        if t.starts_with("version:") {
            has_version = true;
        }
        if let Some(v) = t.strip_prefix("requirements:") {
            req_value = Some(v.trim().to_string());
        }
    }

    if !has_version {
        issues.push(issue(
            OWUI030,
            path,
            line,
            1,
            "Module header is missing a `version:` field. Consider adding e.g. `version: 0.1.0`.",
        ));
    }

    if let Some(reqs) = req_value {
        let unpinned: Vec<&str> = reqs
            .split(',')
            .map(str::trim)
            .filter(|r| !r.is_empty() && !has_version_specifier(r))
            .collect();
        if !unpinned.is_empty() {
            issues.push(issue(
                OWUI031,
                path,
                line,
                1,
                format!(
                    "Unpinned requirements detected: {}. Consider pinning versions with `==` \
                     (e.g. `{}==1.2.3`).",
                    unpinned.join(", "),
                    unpinned[0],
                ),
            ));
        }
    }

    issues
}

fn included(path: &Path, cwd: &Path, include: &[String], exclude: &[String]) -> bool {
    let absolute = normalize_path(path);
    let relative = path
        .strip_prefix(cwd)
        .ok()
        .map(normalize_path)
        .unwrap_or_else(|| absolute.clone());

    if exclude
        .iter()
        .any(|pattern| glob_match(pattern, &relative) || glob_match(pattern, &absolute))
    {
        return false;
    }

    if include.is_empty() {
        return true;
    }

    include
        .iter()
        .any(|pattern| glob_match(pattern, &relative) || glob_match(pattern, &absolute))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn canonical(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .map_err(|err| anyhow!("Failed to resolve {}: {err}", path.display()))
}
