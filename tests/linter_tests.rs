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

#[test]
fn sync_tool_method_warns_owt102() {
    let temp = TempDir::new("owt102");
    let file_path = temp.write(
        "tools.py",
        "from pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    def search(self, query: str) -> str:\n        \"\"\"Searches for something.\"\"\"\n        return query\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWT102" && issue.severity == Severity::Warning)
    );
}

#[test]
fn async_tool_method_does_not_warn_owt102() {
    let temp = TempDir::new("owt102_async");
    let file_path = temp.write(
        "tools.py",
        "from pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def search(self, query: str) -> str:\n        \"\"\"Searches for something.\"\"\"\n        return query\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(!issues.iter().any(|issue| issue.rule_id == "OWT102"));
}

#[test]
fn missing_title_in_header_warns_owui032() {
    let temp = TempDir::new("owui032");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWUI032" && issue.severity == Severity::Warning)
    );
}

#[test]
fn prefixed_module_header_still_triggers_requirement_checks() {
    let temp = TempDir::new("owui031_prefixed_header");
    let file_path = temp.write(
        "tools.py",
        "r\"\"\"\ntitle: My Tool\nversion: 0.1.0\nrequirements: requests\n\"\"\"\nfrom pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWUI031" && issue.severity == Severity::Warning),
        "Expected OWUI031 for unpinned requirements in prefixed module header, got: {issues:?}"
    );
    assert!(!issues.iter().any(|issue| issue.rule_id == "OWUI030"));
    assert!(!issues.iter().any(|issue| issue.rule_id == "OWUI032"));
}

#[test]
fn pipeline_with_pipes_method_does_not_raise_owpl500() {
    let temp = TempDir::new("pipeline_manifold");
    let file_path = temp.write(
        "pipeline.py",
        "from pydantic import BaseModel\n\nclass Pipeline:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.name = \"Manifold\"\n        self.valves = self.Valves()\n\n    def pipes(self) -> list:\n        return [{\"id\": \"model-a\", \"name\": \"Model A\"}]\n\n    async def pipe(self, user_message, model_id, messages, body):\n        return user_message\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(!issues.iter().any(|issue| issue.rule_id == "OWPL500"));
}

#[test]
fn filter_inlet_returning_parenthesized_body_does_not_warn_owf301() {
    let temp = TempDir::new("owf301_parenthesized_return");
    let file_path = temp.write(
        "filter.py",
        "from pydantic import BaseModel\n\nclass Filter:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def inlet(self, body: dict) -> dict:\n        return (body)\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        !issues.iter().any(|issue| issue.rule_id == "OWF301"),
        "Parenthesized `return (body)` should satisfy OWF301, got: {issues:?}"
    );
}

#[test]
fn filter_outlet_without_return_body_does_not_warn() {
    let temp = TempDir::new("owf302_removed");
    let file_path = temp.write(
        "filter.py",
        "from pydantic import BaseModel\n\nclass Filter:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def inlet(self, body: dict) -> dict:\n        return body\n\n    async def outlet(self, body: dict) -> None:\n        pass\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(!issues.iter().any(|issue| issue.rule_id == "OWF302"));
}

#[test]
fn sensitive_valve_field_without_password_type_warns_owui023() {
    let temp = TempDir::new("owui023_basic");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel, Field\n\nclass Tools:\n    class Valves(BaseModel):\n        api_key: str = \"\"\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWUI023" && issue.severity == Severity::Warning),
        "Expected OWUI023 for unmasked api_key field, got: {issues:?}"
    );
}

#[test]
fn sensitive_valve_field_with_password_type_no_owui023() {
    let temp = TempDir::new("owui023_masked");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel, Field\n\nclass Tools:\n    class Valves(BaseModel):\n        api_key: str = Field(\n            default=\"\",\n            json_schema_extra={\"input\": {\"type\": \"password\"}}\n        )\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        !issues.iter().any(|issue| issue.rule_id == "OWUI023"),
        "Should not warn when password type is set, got: {issues:?}"
    );
}

#[test]
fn non_sensitive_valve_field_no_owui023() {
    let temp = TempDir::new("owui023_nonsensitive");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        base_url: str = \"https://example.com\"\n        max_retries: int = 3\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        !issues.iter().any(|issue| issue.rule_id == "OWUI023"),
        "Non-sensitive fields should not trigger OWUI023, got: {issues:?}"
    );
}

#[test]
fn uppercase_sensitive_valve_field_warns_owui023() {
    let temp = TempDir::new("owui023_upper");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        OPENAI_API_KEY: str = \"\"\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWUI023" && issue.severity == Severity::Warning),
        "Expected OWUI023 for OPENAI_API_KEY, got: {issues:?}"
    );
}

#[test]
fn mixed_compliance_valve_fields_only_warns_unmasked() {
    let temp = TempDir::new("owui023_mixed");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel, Field\n\nclass Tools:\n    class Valves(BaseModel):\n        api_key: str = Field(default=\"\", json_schema_extra={\"input\": {\"type\": \"password\"}})\n        secret_token: str = \"\"\n        base_url: str = \"https://example.com\"\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    let owui023_issues: Vec<_> = issues
        .iter()
        .filter(|issue| issue.rule_id == "OWUI023")
        .collect();
    assert_eq!(
        owui023_issues.len(),
        1,
        "Expected exactly 1 OWUI023 (for secret_token), got: {owui023_issues:?}"
    );
    assert!(
        owui023_issues[0].message.contains("secret_token"),
        "OWUI023 should be for secret_token, got: {}",
        owui023_issues[0].message
    );
}

#[test]
fn sensitive_field_with_password_word_in_description_still_warns_owui023() {
    let temp = TempDir::new("owui023_password_word_only");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel, Field\n\nclass Tools:\n    class Valves(BaseModel):\n        api_key: str = Field(default=\"\", description=\"Use your password from provider dashboard\")\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWUI023" && issue.severity == Severity::Warning),
        "Expected OWUI023 when only description mentions password, got: {issues:?}"
    );
}

#[test]
fn tokenizer_like_field_name_does_not_warn_owui023() {
    let temp = TempDir::new("owui023_tokenizer_false_positive");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel\n\nclass Tools:\n    class Valves(BaseModel):\n        tokenizer_model: str = \"gpt2\"\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        !issues.iter().any(|issue| issue.rule_id == "OWUI023"),
        "tokenizer-like names should not trigger OWUI023, got: {issues:?}"
    );
}

#[test]
fn token_count_toggle_does_not_warn_owui023() {
    let temp = TempDir::new("owui023_token_count_toggle");
    let file_path = temp.write(
        "filter.py",
        "\"\"\"\ntitle: Token Count Filter\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel, Field\n\nclass Filter:\n    class Valves(BaseModel):\n        SHOW_TOKEN_COUNT: bool = Field(default=True, description=\"Show token count\")\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def inlet(self, body: dict) -> dict:\n        return body\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        !issues.iter().any(|issue| issue.rule_id == "OWUI023"),
        "token count toggles should not trigger OWUI023, got: {issues:?}"
    );
}

#[test]
fn password_word_in_non_type_field_does_not_count_as_masking() {
    let temp = TempDir::new("owui023_password_placeholder_only");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel, Field\n\nclass Tools:\n    class Valves(BaseModel):\n        api_key: str = Field(default=\"\", json_schema_extra={\"input\": {\"type\": \"text\", \"placeholder\": \"password\"}})\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWUI023" && issue.severity == Severity::Warning),
        "Expected OWUI023 when input type is not password, got: {issues:?}"
    );
}

#[test]
fn single_quoted_password_input_type_counts_as_masked() {
    let temp = TempDir::new("owui023_single_quote_password_type");
    let file_path = temp.write(
        "tools.py",
        "\"\"\"\ntitle: My Tool\nversion: 0.1.0\n\"\"\"\nfrom pydantic import BaseModel, Field\n\nclass Tools:\n    class Valves(BaseModel):\n        api_key: str = Field(default='', json_schema_extra={'input': {'type': 'password'}})\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def do_thing(self, x: str) -> str:\n        \"\"\"Does a thing.\"\"\"\n        return x\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        !issues.iter().any(|issue| issue.rule_id == "OWUI023"),
        "Expected no OWUI023 when input type is password (single quotes), got: {issues:?}"
    );
}

#[test]
fn user_valves_only_satisfies_common_valve_rules() {
    let temp = TempDir::new("user_valves_only");
    let file_path = temp.write(
        "filter.py",
        "from pydantic import BaseModel\n\nclass Filter:\n    class UserValves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.user_valves = self.UserValves()\n\n    async def inlet(self, body: dict, __user__: dict) -> dict:\n        return body\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(!issues.iter().any(|issue| issue.rule_id == "OWUI020"));
    assert!(!issues.iter().any(|issue| issue.rule_id == "OWUI021"));
    assert!(!issues.iter().any(|issue| issue.rule_id == "OWUI022"));
}

#[test]
fn user_valves_without_basemodel_warns_owui021() {
    let temp = TempDir::new("user_valves_no_basemodel");
    let file_path = temp.write(
        "filter.py",
        "class Filter:\n    class UserValves:\n        pass\n\n    def __init__(self):\n        self.user_valves = self.UserValves()\n\n    async def inlet(self, body: dict, __user__: dict) -> dict:\n        return body\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(issues.iter().any(|issue| issue.rule_id == "OWUI021"));
}

#[test]
fn filter_inlet_without_body_arg_warns_signature_rule() {
    let temp = TempDir::new("owf303_inlet_missing_body");
    let file_path = temp.write(
        "filter.py",
        "from pydantic import BaseModel\n\nclass Filter:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def inlet(self, payload: dict) -> dict:\n        return payload\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWF303" && issue.severity == Severity::Warning),
        "Expected OWF303 for inlet missing body arg, got: {issues:?}"
    );
}

#[test]
fn filter_stream_without_event_arg_warns_signature_rule() {
    let temp = TempDir::new("owf304_stream_missing_event");
    let file_path = temp.write(
        "filter.py",
        "from pydantic import BaseModel\n\nclass Filter:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def stream(self, chunk: dict):\n        return chunk\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWF304" && issue.severity == Severity::Warning),
        "Expected OWF304 for stream missing event arg, got: {issues:?}"
    );
}

#[test]
fn action_without_body_arg_warns_signature_rule() {
    let temp = TempDir::new("owa402_action_missing_body");
    let file_path = temp.write(
        "action.py",
        "from pydantic import BaseModel\n\nclass Action:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def action(self, payload: dict):\n        return payload\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWA402" && issue.severity == Severity::Warning),
        "Expected OWA402 for action missing body arg, got: {issues:?}"
    );
}

#[test]
fn filter_reserved_args_with_body_event_names_do_not_warn_signature_rules() {
    let temp = TempDir::new("reserved_args_signature_ok");
    let file_path = temp.write(
        "filter.py",
        "from pydantic import BaseModel\n\nclass Filter:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def inlet(self, body: dict, __user__: dict, __event_emitter__=None) -> dict:\n        return body\n\n    async def stream(self, event: dict, __metadata__: dict):\n        return event\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(!issues.iter().any(|issue| issue.rule_id == "OWF303"));
    assert!(!issues.iter().any(|issue| issue.rule_id == "OWF304"));
}

#[test]
fn user_valves_not_initialized_warns_owui022() {
    let temp = TempDir::new("user_valves_not_initialized");
    let file_path = temp.write(
        "filter.py",
        "from pydantic import BaseModel\n\nclass Filter:\n    class UserValves(BaseModel):\n        pass\n\n    def __init__(self):\n        pass\n\n    async def inlet(self, body: dict, __user__: dict) -> dict:\n        return body\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        !issues.iter().any(|issue| issue.rule_id == "OWUI022"),
        "UserValves should not require self.user_valves initialization, got: {issues:?}"
    );
}

#[test]
fn valves_not_initialized_still_warns_owui022() {
    let temp = TempDir::new("valves_not_initialized");
    let file_path = temp.write(
        "pipe.py",
        "from pydantic import BaseModel\n\nclass Pipe:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        pass\n\n    async def pipe(self, body: dict) -> dict:\n        return body\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues.iter().any(|issue| issue.rule_id == "OWUI022"),
        "Valves should still require self.valves initialization, got: {issues:?}"
    );
}

#[test]
fn sensitive_user_valve_field_without_password_type_warns_owui023() {
    let temp = TempDir::new("owui023_user_valves");
    let file_path = temp.write(
        "filter.py",
        "from pydantic import BaseModel\n\nclass Filter:\n    class UserValves(BaseModel):\n        api_key: str = \"\"\n\n    def __init__(self):\n        self.user_valves = self.UserValves()\n\n    async def inlet(self, body: dict, __user__: dict) -> dict:\n        return body\n",
    );

    let config = Config::default();
    let files = discover_python_files(&[file_path], &config.include, &config.exclude)
        .expect("discovery should work");
    let (issues, _) = lint_files(&files, &config);

    assert!(
        issues
            .iter()
            .any(|issue| issue.rule_id == "OWUI023" && issue.severity == Severity::Warning),
        "Expected OWUI023 for unmasked user-scoped api_key field, got: {issues:?}"
    );
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
