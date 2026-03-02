use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use crate::models::{Issue, LintSummary, Severity};
use crate::rules::{all_rules, rule_doc};

pub fn format_text(issues: &[Issue], summary: &LintSummary) -> String {
    let mut lines = Vec::with_capacity((issues.len() * 4) + 3);

    for issue in issues {
        lines.push(format!(
            "{}:{}:{}: {} {} {}",
            issue.path.display(),
            issue.line,
            issue.column,
            issue.severity,
            issue.rule_id,
            issue.message
        ));

        if let Some(rule) = rule_doc(issue.rule_id) {
            lines.push(format!("  help: {}", rule.summary));
            lines.push(format!("  fix: {}", rule.remediation));
        }
        lines.push(String::new());
    }

    lines.push(format!(
        "Scanned {} file(s), found {} error(s), {} warning(s).",
        summary.files_scanned, summary.errors, summary.warnings
    ));

    if !issues.is_empty() {
        lines.push(format!("Rules triggered: {}", format_rule_counts(issues)));
    }

    lines.join("\n")
}

pub fn format_json(issues: &[Issue], summary: &LintSummary) -> String {
    #[derive(Serialize)]
    struct JsonIssue<'a> {
        rule_id: &'a str,
        severity: Severity,
        message: &'a str,
        path: String,
        line: usize,
        column: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        rule_title: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        rule_summary: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        remediation: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        help_url: Option<&'a str>,
    }

    #[derive(Serialize)]
    struct JsonRule<'a> {
        id: &'a str,
        default_severity: Severity,
        title: &'a str,
        summary: &'a str,
        remediation: &'a str,
        help_url: &'a str,
    }

    let unique_rules: BTreeSet<&str> = issues.iter().map(|issue| issue.rule_id).collect();

    let payload = json!({
        "summary": summary,
        "rules": unique_rules
            .iter()
            .filter_map(|rule_id| rule_doc(rule_id))
            .map(|rule| JsonRule {
                id: rule.id,
                default_severity: rule.default_severity,
                title: rule.title,
                summary: rule.summary,
                remediation: rule.remediation,
                help_url: rule.help_url,
            })
            .collect::<Vec<_>>(),
        "issues": issues
            .iter()
            .map(|issue| {
                let rule = rule_doc(issue.rule_id);
                JsonIssue {
                    rule_id: issue.rule_id,
                    severity: issue.severity,
                    message: &issue.message,
                    path: issue.path.display().to_string(),
                    line: issue.line,
                    column: issue.column,
                    rule_title: rule.map(|value| value.title),
                    rule_summary: rule.map(|value| value.summary),
                    remediation: rule.map(|value| value.remediation),
                    help_url: rule.map(|value| value.help_url),
                }
            })
            .collect::<Vec<_>>()
    });

    serde_json::to_string_pretty(&payload).expect("json formatting should succeed")
}

pub fn format_github(issues: &[Issue], summary: &LintSummary) -> String {
    let mut lines = Vec::with_capacity(issues.len() + 1);

    for issue in issues {
        let level = if issue.severity == Severity::Error {
            "error"
        } else {
            "warning"
        };
        let rule = rule_doc(issue.rule_id);
        let message = escape_github(&format!(
            "{}: {}{}",
            issue.rule_id,
            issue.message,
            rule.map(|value| format!(" Fix: {}", value.remediation))
                .unwrap_or_default()
        ));
        let title = escape_github(rule.map(|value| value.title).unwrap_or("owui-lint"));
        let file_path = escape_github(&issue.path.display().to_string());

        lines.push(format!(
            "::{level} file={file_path},line={},col={},title={title}::{message}",
            issue.line, issue.column
        ));
    }

    lines.push(format!(
        "owui-lint: scanned {} file(s), {} error(s), {} warning(s).",
        summary.files_scanned, summary.errors, summary.warnings
    ));
    if !issues.is_empty() {
        lines.push(format!(
            "owui-lint: rules triggered -> {}",
            format_rule_counts(issues)
        ));
    }

    lines.join("\n")
}

pub fn format_sarif(issues: &[Issue], summary: &LintSummary, version: &str) -> String {
    let unique_rules: BTreeSet<&str> = issues.iter().map(|issue| issue.rule_id).collect();
    let rules = unique_rules
        .iter()
        .map(|rule_id| {
            if let Some(rule) = rule_doc(rule_id) {
                json!({
                    "id": rule.id,
                    "name": rule.id,
                    "shortDescription": { "text": rule.title },
                    "fullDescription": { "text": rule.summary },
                    "help": {
                        "text": format!("{} See: {}", rule.remediation, rule.help_url)
                    },
                    "helpUri": rule.help_url,
                    "defaultConfiguration": {
                        "level": sarif_level(rule.default_severity)
                    }
                })
            } else {
                json!({
                    "id": rule_id,
                    "name": rule_id,
                    "shortDescription": { "text": rule_id }
                })
            }
        })
        .collect::<Vec<Value>>();

    let results = issues
        .iter()
        .map(|issue| {
            json!({
                "ruleId": issue.rule_id,
                "level": sarif_level(issue.severity),
                "message": { "text": issue.message },
                "locations": [
                    {
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": to_relative_uri(&issue.path)
                            },
                            "region": {
                                "startLine": issue.line,
                                "startColumn": issue.column
                            }
                        }
                    }
                ]
            })
        })
        .collect::<Vec<Value>>();

    let payload = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "owui-lint",
                        "semanticVersion": version,
                        "informationUri": "https://github.com/open-webui/open-webui",
                        "rules": rules
                    }
                },
                "results": results,
                "properties": {
                    "files_scanned": summary.files_scanned,
                    "errors": summary.errors,
                    "warnings": summary.warnings
                }
            }
        ]
    });

    serde_json::to_string_pretty(&payload).expect("sarif serialization should succeed")
}

pub fn format_rule_list_text() -> String {
    let mut lines = Vec::new();
    lines.push(format!("Available rules: {}", all_rules().len()));
    lines.push(String::new());

    for rule in all_rules() {
        lines.push(format!(
            "{} [{}] {}",
            rule.id, rule.default_severity, rule.title
        ));
        lines.push(format!("  {}", rule.summary));
        lines.push(format!("  fix: {}", rule.remediation));
        lines.push(format!("  docs: {}", rule.help_url));
        lines.push(String::new());
    }

    lines.join("\n")
}

pub fn format_rule_list_json() -> String {
    #[derive(Serialize)]
    struct JsonRule<'a> {
        id: &'a str,
        default_severity: Severity,
        title: &'a str,
        summary: &'a str,
        remediation: &'a str,
        help_url: &'a str,
    }

    let payload = json!({
        "rules": all_rules()
            .iter()
            .map(|rule| JsonRule {
                id: rule.id,
                default_severity: rule.default_severity,
                title: rule.title,
                summary: rule.summary,
                remediation: rule.remediation,
                help_url: rule.help_url,
            })
            .collect::<Vec<_>>()
    });

    serde_json::to_string_pretty(&payload).expect("json formatting should succeed")
}

pub fn format_rule_explanation(rule_id: &str) -> Option<String> {
    let rule = rule_doc(rule_id)?;
    Some(format!(
        "{} [{}] {}\n{}\nFix: {}\nDocs: {}",
        rule.id, rule.default_severity, rule.title, rule.summary, rule.remediation, rule.help_url
    ))
}

fn sarif_level(severity: Severity) -> &'static str {
    if severity == Severity::Error {
        "error"
    } else {
        "warning"
    }
}

fn to_relative_uri(path: &Path) -> String {
    let cwd = std::env::current_dir().ok();
    if let Some(cwd) = cwd
        && let Ok(relative) = path.canonicalize().and_then(|absolute| {
            absolute
                .strip_prefix(&cwd)
                .map(|value| value.to_path_buf())
                .map_err(std::io::Error::other)
        })
    {
        return relative.to_string_lossy().replace('\\', "/");
    }
    path.canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .replace('\\', "/")
}

fn escape_github(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(':', "%3A")
        .replace(',', "%2C")
}

fn format_rule_counts(issues: &[Issue]) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for issue in issues {
        *counts.entry(issue.rule_id).or_insert(0) += 1;
    }

    counts
        .into_iter()
        .map(|(rule, count)| format!("{rule} ({count})"))
        .collect::<Vec<_>>()
        .join(", ")
}
