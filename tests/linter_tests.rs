use std::fs;
use std::path::PathBuf;

use owui_lint::config::Config;
use owui_lint::linter::{discover_python_files, lint_files};
use owui_lint::models::{Severity, SeverityOverride};

#[test]
fn reports_syntax_error() {
    let temp = TempDir::new("syntax");
    let file_path = temp.write(
        "broken.py",
        "class Pipe:\n    async def pipe(self, body: dict):\n        return body\n    def oops(\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, summary) = lint_files(&files, &config);

    assert_eq!(summary.errors, 1);
    assert!(issues.iter().any(|issue| issue.rule_id == "OWUI001"));
}

#[test]
fn pipe_with_filter_methods_is_warning() {
    let temp = TempDir::new("pipe_filter");
    let file_path = temp.write(
        "pipe.py",
        "from pydantic import BaseModel\n\nclass Pipe:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def inlet(self, body: dict) -> dict:\n        return body\n\n    async def pipe(self, body: dict) -> str:\n        return \"ok\"\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWP201" && issue.severity == Severity::Warning)
    );
}

#[test]
fn tools_method_without_docstring_warns() {
    let temp = TempDir::new("tools_doc");
    let file_path = temp.write(
        "tools.py",
        "from pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def get_weather(self, city: str) -> str:\n        return city\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWT101" && issue.severity == Severity::Warning)
    );
}

#[test]
fn rule_override_can_disable_warning() {
    let temp = TempDir::new("rule_override");
    let file_path = temp.write(
        "tools.py",
        "from pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def get_weather(self, city: str) -> str:\n        return city\n",
    );

    let mut config = Config::default();
    config
        .rule_overrides
        .insert("OWT101".to_string(), SeverityOverride::Off);

    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, summary) = lint_files(&files, &config);

    assert!(!issues.iter().any(|issue| issue.rule_id == "OWT101"));
    assert_eq!(summary.warnings, 0);
}

#[test]
fn filter_with_stream_only_does_not_raise_owf300() {
    let temp = TempDir::new("filter_stream");
    let file_path = temp.write(
        "filter.py",
        "from pydantic import BaseModel\n\nclass Filter:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def stream(self, event):\n        return event\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(!issues.iter().any(|issue| issue.rule_id == "OWF300"));
}

#[test]
fn pipeline_with_filter_hooks_does_not_raise_owpl500() {
    let temp = TempDir::new("pipeline_filter_hooks");
    let file_path = temp.write(
        "pipeline.py",
        "from pydantic import BaseModel\n\nclass Pipeline:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.name = \"Filter Pipeline\"\n        self.valves = self.Valves()\n\n    async def inlet(self, body: dict, user: dict) -> dict:\n        return body\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(!issues.iter().any(|issue| issue.rule_id == "OWPL500"));
}

#[test]
fn multiline_pipe_signature_does_not_raise_owp200() {
    let temp = TempDir::new("multiline_pipe");
    let file_path = temp.write(
        "pipe.py",
        "from pydantic import BaseModel\n\nclass Pipe:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def pipe(\n        self,\n        body: dict,\n    ) -> dict:\n        return body\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(!issues.iter().any(|issue| issue.rule_id == "OWP200"));
}

#[test]
fn missing_version_in_header_warns_owui030() {
    let temp = TempDir::new("owui030");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nrequirements: requests==2.31.0\n\"\"\"\nfrom pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWUI030" && issue.severity == Severity::Warning)
    );
    assert!(!issues.iter().any(|issue| issue.rule_id == "OWUI031"));
}

#[test]
fn unpinned_requirements_in_header_warns_owui031() {
    let temp = TempDir::new("owui031");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 0.1.0\nrequirements: requests, httpx==0.27.0\n\"\"\"\nfrom pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWUI031" && issue.severity == Severity::Warning)
    );
    assert!(!issues.iter().any(|issue| issue.rule_id == "OWUI030"));
}

#[test]
fn pinned_requirements_and_version_no_owui030_031() {
    let temp = TempDir::new("owui03x_clean");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 1.2.3\nrequirements: requests==2.31.0, httpx==0.27.0\n\"\"\"\nfrom pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(!issues.iter().any(|issue| issue.rule_id == "OWUI030"));
    assert!(!issues.iter().any(|issue| issue.rule_id == "OWUI031"));
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
