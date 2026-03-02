use std::collections::BTreeSet;
use std::path::Path;

use owui_lint::models::Severity;
use owui_lint::rules::{OWP200, OWT101, all_rules, is_known_rule, issue};

#[test]
fn rule_catalog_has_unique_ids() {
    let mut seen = BTreeSet::new();
    for rule in all_rules() {
        assert!(seen.insert(rule.id), "duplicate rule id: {}", rule.id);
    }
}

#[test]
fn issue_uses_default_error_severity() {
    let finding = issue(OWP200, Path::new("pipe.py"), 10, 2, "test message");
    assert_eq!(finding.severity, Severity::Error);
}

#[test]
fn issue_uses_default_warning_severity() {
    let finding = issue(OWT101, Path::new("tools.py"), 8, 3, "test message");
    assert_eq!(finding.severity, Severity::Warning);
}

#[test]
fn known_rule_detection_works() {
    assert!(is_known_rule(OWP200));
    assert!(!is_known_rule("OWX999"));
}
