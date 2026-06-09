use std::fs;
use std::path::{Path, PathBuf};

use owui_lint::config::Config;
use owui_lint::linter::lint_source;
use owui_lint::models::Severity;

fn fixture(name: &str) -> (PathBuf, String) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/owsec")
        .join(name);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {} should be readable: {err}", path.display()));
    (path, source)
}

fn security_config() -> Config {
    Config {
        security: true,
        ..Config::default()
    }
}

fn owsec001_count(name: &str, config: &Config) -> usize {
    let (path, source) = fixture(name);
    lint_source(&path, &source, config)
        .iter()
        .filter(|issue| issue.rule_id == "OWSEC001")
        .count()
}

#[test]
fn malicious_fixtures_are_flagged_under_security_profile() {
    let config = security_config();
    for name in [
        "module_level_subprocess.py",
        "init_network.py",
        "valves_field_default_eval.py",
    ] {
        assert!(
            owsec001_count(name, &config) >= 1,
            "{name}: expected OWSEC001 to fire on import-time execution"
        );
    }
}

#[test]
fn owsec001_is_an_error() {
    let (path, source) = fixture("module_level_subprocess.py");
    let issue = lint_source(&path, &source, &security_config())
        .into_iter()
        .find(|issue| issue.rule_id == "OWSEC001")
        .expect("OWSEC001 should fire");
    assert_eq!(issue.severity, Severity::Error);
}

#[test]
fn method_scoped_calls_do_not_fire() {
    // Same dangerous call, but only inside a tool method → not import-time.
    assert_eq!(
        owsec001_count("clean_method_only.py", &security_config()),
        0
    );
}

#[test]
fn owsec_is_off_by_default() {
    // Without the security profile, OWSEC must not interfere — no findings at all.
    let config = Config::default();
    for name in [
        "module_level_subprocess.py",
        "init_network.py",
        "valves_field_default_eval.py",
    ] {
        assert_eq!(
            owsec001_count(name, &config),
            0,
            "{name}: OWSEC001 must stay silent when the security profile is off"
        );
    }
}

#[test]
fn scope_variation_same_call_differs_by_location() {
    let import_time = "import subprocess\nsubprocess.run([\"id\"])\n\nclass Tools:\n    class Valves:\n        pass\n    async def t(self, x: str) -> str:\n        return x\n";
    let method_only = "import subprocess\n\nclass Tools:\n    class Valves:\n        pass\n    async def t(self, x: str) -> str:\n        return subprocess.run([x])\n";

    let config = security_config();
    let count = |src: &str| {
        lint_source(Path::new("/virtual/tools.py"), src, &config)
            .iter()
            .filter(|issue| issue.rule_id == "OWSEC001")
            .count()
    };

    assert_eq!(count(import_time), 1, "module-level subprocess should fire");
    assert_eq!(
        count(method_only),
        0,
        "method-body subprocess should not fire"
    );
}
