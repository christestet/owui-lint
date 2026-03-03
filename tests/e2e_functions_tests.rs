/// End-to-end tests against the real example files in `functions/functions/`.
///
/// These tests run the linter on actual Open WebUI extension files from the
/// official open-webui/functions repository, verifying that the linter produces
/// the expected diagnostics (or none) for each file.
use std::path::PathBuf;

use owui_lint::config::Config;
use owui_lint::linter::{discover_python_files, lint_files};
use owui_lint::models::Severity;

fn functions_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("functions/functions")
        .join(relative)
}

macro_rules! lint_file {
    ($relative:expr) => {{
        if std::env::var("CI").is_ok() || !functions_path("").exists() {
            println!("Skipping E2E test in CI or because the 'functions' repository is missing.");
            return;
        }
        let path = functions_path($relative);
        assert!(path.exists(), "Test fixture not found: {}", path.display());
        let config = Config::default();
        let files = discover_python_files(&[path], &config.include, &config.exclude)
            .expect("file discovery should succeed");
        lint_files(&files, &config)
    }};
}

// ---------------------------------------------------------------------------
// Action: actions/example/main.py
// ---------------------------------------------------------------------------

/// The official example action is well-formed and should produce no issues.
#[test]
fn action_example_is_clean() {
    let (issues, summary) = lint_file!("actions/example/main.py");
    assert_eq!(
        summary.errors,
        0,
        "unexpected errors: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        summary.warnings,
        0,
        "unexpected warnings: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect::<Vec<_>>()
    );
}

/// The example action must not trigger the async check (action() is already async).
#[test]
fn action_example_no_owa401() {
    let (issues, _) = lint_file!("actions/example/main.py");
    assert!(
        !issues.iter().any(|i| i.rule_id == "OWA401"),
        "action() is already async – OWA401 must not fire"
    );
}

// ---------------------------------------------------------------------------
// Filter: filters/context_clip/main.py
// ---------------------------------------------------------------------------

/// context_clip is a minimal, correct filter and should produce no issues.
#[test]
fn filter_context_clip_is_clean() {
    let (issues, summary) = lint_file!("filters/context_clip/main.py");
    assert_eq!(
        summary.errors,
        0,
        "unexpected errors: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        summary.warnings,
        0,
        "unexpected warnings: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect::<Vec<_>>()
    );
}

/// context_clip has a proper `inlet` that returns body – OWF301 must not fire.
#[test]
fn filter_context_clip_no_owf301() {
    let (issues, _) = lint_file!("filters/context_clip/main.py");
    assert!(!issues.iter().any(|i| i.rule_id == "OWF301"));
}

// ---------------------------------------------------------------------------
// Filter: filters/max_turns/main.py
// ---------------------------------------------------------------------------

/// max_turns has both inlet and outlet, both returning body – no issues expected.
#[test]
fn filter_max_turns_is_clean() {
    let (issues, summary) = lint_file!("filters/max_turns/main.py");
    assert_eq!(
        summary.errors,
        0,
        "unexpected errors: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        summary.warnings,
        0,
        "unexpected warnings: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect::<Vec<_>>()
    );
}

/// max_turns outlet returns body (it's a valid pattern) – since OWF302 is removed,
/// no warning must fire for a correct outlet implementation.
#[test]
fn filter_max_turns_no_owf302_false_positive() {
    let (issues, _) = lint_file!("filters/max_turns/main.py");
    assert!(
        !issues.iter().any(|i| i.rule_id == "OWF302"),
        "OWF302 was removed – must never appear"
    );
}

// ---------------------------------------------------------------------------
// Filter: filters/dynamic_vision_router/main.py
// ---------------------------------------------------------------------------

/// dynamic_vision_router has a single async inlet that returns body – no issues.
#[test]
fn filter_dynamic_vision_router_is_clean() {
    let (issues, summary) = lint_file!("filters/dynamic_vision_router/main.py");
    assert_eq!(
        summary.errors,
        0,
        "unexpected errors: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        summary.warnings,
        0,
        "unexpected warnings: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Filter: filters/summarizer/main.py
// ---------------------------------------------------------------------------

/// summarizer has async inlet (returns body) and async outlet (returns body).
/// Since OWF302 was removed (outlet is void per spec), no warning must fire.
#[test]
fn filter_summarizer_no_errors() {
    let (issues, summary) = lint_file!("filters/summarizer/main.py");
    assert_eq!(
        summary.errors,
        0,
        "unexpected errors: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect::<Vec<_>>()
    );
}

#[test]
fn filter_summarizer_no_owf301() {
    let (issues, _) = lint_file!("filters/summarizer/main.py");
    assert!(
        !issues.iter().any(|i| i.rule_id == "OWF301"),
        "summarizer inlet returns body – OWF301 must not fire"
    );
}

#[test]
fn filter_summarizer_no_owf302() {
    let (issues, _) = lint_file!("filters/summarizer/main.py");
    assert!(
        !issues.iter().any(|i| i.rule_id == "OWF302"),
        "OWF302 was removed – must never appear"
    );
}

// ---------------------------------------------------------------------------
// Pipe: pipes/anthropic/main.py  (manifold)
// ---------------------------------------------------------------------------

/// anthropic pipe is a manifold (pipes() + pipe()). OWP202 fires because pipe()
/// is synchronous – this is a known, intentional pattern in many manifold pipes.
#[test]
fn pipe_anthropic_warns_owp202() {
    let (issues, _) = lint_file!("pipes/anthropic/main.py");
    assert!(
        issues
            .iter()
            .any(|i| i.rule_id == "OWP202" && i.severity == Severity::Warning),
        "anthropic pipe() is not async – OWP202 must fire"
    );
}

/// The anthropic manifold defines pipes() – OWPL500 / OWP200 must not fire.
#[test]
fn pipe_anthropic_no_owp200() {
    let (issues, _) = lint_file!("pipes/anthropic/main.py");
    assert!(
        !issues.iter().any(|i| i.rule_id == "OWP200"),
        "anthropic has pipe() method – OWP200 must not fire"
    );
}

/// anthropic pipe has no errors, only the async warning.
#[test]
fn pipe_anthropic_no_errors() {
    let (issues, summary) = lint_file!("pipes/anthropic/main.py");
    assert_eq!(
        summary.errors,
        0,
        "unexpected errors: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Pipe: pipes/openai/main.py  (manifold)
// ---------------------------------------------------------------------------

/// openai pipe is a manifold (pipes() + pipe()). pipe() is sync → OWP202.
#[test]
fn pipe_openai_warns_owp202() {
    let (issues, _) = lint_file!("pipes/openai/main.py");
    assert!(
        issues
            .iter()
            .any(|i| i.rule_id == "OWP202" && i.severity == Severity::Warning),
        "openai pipe() is not async – OWP202 must fire"
    );
}

/// openai pipe has no errors.
#[test]
fn pipe_openai_no_errors() {
    let (issues, summary) = lint_file!("pipes/openai/main.py");
    assert_eq!(
        summary.errors,
        0,
        "unexpected errors: {:?}",
        issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Known parser limitation: filters/agent_hotswap/main.py
// ---------------------------------------------------------------------------

/// agent_hotswap contains a multiline string whose content has lines at column 0.
/// The indent-based parser incorrectly pops the Filter class context when it
/// encounters those column-0 lines, causing inlet/outlet not to be attributed to
/// the Filter class. This is a documented false positive.
///
/// If this test starts FAILING (i.e. no OWF300 appears), the parser has been
/// improved and this test should be updated to assert zero errors instead.
#[test]
fn filter_agent_hotswap_known_false_positive_owf300() {
    let (issues, _) = lint_file!("filters/agent_hotswap/main.py");
    assert!(
        issues.iter().any(|i| i.rule_id == "OWF300"),
        "OWF300 is a known false positive for agent_hotswap (multiline string with \
         column-0 content confuses the indent-based parser). If this assertion \
         fails, the parser has been fixed – update the test to assert no errors."
    );
}
