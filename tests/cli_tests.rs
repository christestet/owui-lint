use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use owui_lint::run;

#[test]
fn cli_json_output_and_exit_code() {
    let temp = TempDir::new("cli_json");
    let file_path = temp.write(
        "pipe.py",
        "class Pipe:\n    async def pipe(self, body: dict) -> str:\n        return \"ok\"\n",
    );
    let output_path = temp.path.join("report.json");

    let exit_code = run([
        "owui-lint",
        file_path.to_str().expect("path should be utf-8"),
        "--format",
        "json",
        "--output",
        output_path.to_str().expect("path should be utf-8"),
        "--fail-on",
        "none",
    ]);

    let payload: Value = serde_json::from_str(
        &fs::read_to_string(output_path).expect("json output should be readable"),
    )
    .expect("json output should be valid");

    assert_eq!(exit_code, 0);
    assert_eq!(payload["summary"]["files_scanned"].as_u64(), Some(1));
    assert!(payload["issues"].is_array());
}

#[test]
fn cli_fails_on_error() {
    let temp = TempDir::new("cli_error");
    let file_path = temp.write(
        "broken.py",
        "class Pipe:\n    def pipe(self, body):\n        return body\n    def x(\n",
    );
    let output_path = temp.path.join("report.txt");

    let exit_code = run([
        "owui-lint",
        file_path.to_str().expect("path should be utf-8"),
        "--format",
        "text",
        "--output",
        output_path.to_str().expect("path should be utf-8"),
        "--fail-on",
        "error",
    ]);

    assert_eq!(exit_code, 1);
}

#[test]
fn cli_sarif_output() {
    let temp = TempDir::new("cli_sarif");
    let file_path = temp.write(
        "tools.py",
        "class Tools:\n    async def fetch(self, query: str) -> str:\n        return query\n",
    );
    let output_path = temp.path.join("owui-lint.sarif");

    let exit_code = run([
        "owui-lint",
        file_path.to_str().expect("path should be utf-8"),
        "--format",
        "sarif",
        "--output",
        output_path.to_str().expect("path should be utf-8"),
        "--fail-on",
        "none",
    ]);

    let payload: Value = serde_json::from_str(
        &fs::read_to_string(output_path).expect("sarif output should be readable"),
    )
    .expect("sarif output should be valid json");

    assert_eq!(exit_code, 0);
    assert_eq!(payload["version"].as_str(), Some("2.1.0"));
    assert_eq!(
        payload["runs"][0]["tool"]["driver"]["name"].as_str(),
        Some("owui-lint")
    );
    assert!(payload["runs"][0]["results"].is_array());
}

#[test]
fn cli_rules_json_output() {
    let temp = TempDir::new("cli_rules");
    let output_path = temp.path.join("rules.json");

    let exit_code = run([
        "owui-lint",
        "rules",
        "--format",
        "json",
        "--output",
        output_path.to_str().expect("path should be utf-8"),
    ]);

    let payload: Value = serde_json::from_str(
        &fs::read_to_string(output_path).expect("rules output should be readable"),
    )
    .expect("rules output should be valid json");

    assert_eq!(exit_code, 0);
    assert!(payload["rules"].is_array());
    assert!(
        payload["rules"]
            .as_array()
            .expect("rules should be array")
            .len()
            >= 10
    );
}

#[test]
fn cli_explain_rule_output() {
    let temp = TempDir::new("cli_explain");
    let output_path = temp.path.join("explain.txt");

    let exit_code = run([
        "owui-lint",
        "explain",
        "OWT101",
        "--output",
        output_path.to_str().expect("path should be utf-8"),
    ]);

    let explain_output =
        fs::read_to_string(output_path).expect("explain output should be readable text");

    assert_eq!(exit_code, 0);
    assert!(explain_output.contains("OWT101"));
    assert!(explain_output.contains("Fix:"));
}

#[test]
fn cli_text_output_contains_fix_guidance() {
    let temp = TempDir::new("cli_text_guidance");
    let file_path = temp.write(
        "tools.py",
        "from pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def get_weather(self, city: str) -> str:\n        return city\n",
    );
    let output_path = temp.path.join("report.txt");

    let exit_code = run([
        "owui-lint",
        file_path.to_str().expect("path should be utf-8"),
        "--format",
        "text",
        "--output",
        output_path.to_str().expect("path should be utf-8"),
        "--fail-on",
        "none",
    ]);

    let report = fs::read_to_string(output_path).expect("text report should be readable");

    assert_eq!(exit_code, 0);
    assert!(report.contains("OWT101"));
    assert!(report.contains("help:"));
    assert!(report.contains("fix:"));
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "owui_lint_{prefix}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("temporary directory should be created");
        Self { path }
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, content).expect("test file should be written");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
