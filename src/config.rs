use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use crate::models::SeverityOverride;
use crate::util::{count_indent, strip_inline_comment};

const DEFAULT_EXCLUDES: [&str; 3] = [".git/**", ".venv/**", "**/__pycache__/**"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub rule_overrides: BTreeMap<String, SeverityOverride>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            include: vec!["**/*.py".to_string()],
            exclude: DEFAULT_EXCLUDES
                .iter()
                .map(|value| value.to_string())
                .collect(),
            rule_overrides: BTreeMap::new(),
        }
    }
}

pub fn load_config(config_path: Option<&Path>) -> Result<Config> {
    if let Some(path) = config_path {
        if !path.exists() {
            return Err(anyhow!("Config not found: {}", path.display()));
        }
        return parse_yaml_config(&fs::read_to_string(path)?)
            .map_err(|err| anyhow!("Invalid config {}: {err}", path.display()));
    }

    if let Some(default_path) = find_default_config_path() {
        return parse_yaml_config(&fs::read_to_string(&default_path)?)
            .map_err(|err| anyhow!("Invalid config {}: {err}", default_path.display()));
    }

    Ok(Config::default())
}

fn find_default_config_path() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let config_yml = cwd.join("config.yml");
    if config_yml.exists() {
        return Some(config_yml);
    }

    let config_yaml = cwd.join("config.yaml");
    if config_yaml.exists() {
        return Some(config_yaml);
    }

    let yml = cwd.join("owui-lint.yml");
    if yml.exists() {
        return Some(yml);
    }

    let yaml = cwd.join("owui-lint.yaml");
    if yaml.exists() {
        return Some(yaml);
    }

    None
}

fn parse_yaml_config(input: &str) -> std::result::Result<Config, String> {
    let mut config = Config::default();

    let mut current_section = String::new();
    let mut lint_list_key = String::new();

    for raw_line in input.lines() {
        let line_without_comment = strip_inline_comment(raw_line);
        if line_without_comment.trim().is_empty() {
            continue;
        }

        let indent = count_indent(line_without_comment);
        let line = line_without_comment.trim();

        if indent == 0 {
            lint_list_key.clear();
            if let Some(section) = line.strip_suffix(':') {
                current_section = section.trim().to_string();
            }
            continue;
        }

        if current_section == "lint" {
            if indent <= 2 && line.ends_with(':') {
                lint_list_key = line.trim_end_matches(':').trim().to_string();
                if lint_list_key == "include" {
                    config.include.clear();
                } else if lint_list_key == "exclude" {
                    config.exclude.clear();
                }
                continue;
            }

            if line.starts_with('-') && (lint_list_key == "include" || lint_list_key == "exclude") {
                let value = unquote(line.trim_start_matches('-').trim());
                if value.is_empty() {
                    continue;
                }
                if lint_list_key == "include" {
                    config.include.push(value.to_string());
                } else {
                    config.exclude.push(value.to_string());
                }
            }
            continue;
        }

        if current_section == "rules" {
            if let Some((key, value)) = split_key_value(line) {
                let key = key.trim().to_ascii_uppercase();
                if key.is_empty() {
                    continue;
                }
                if let Some(severity) = SeverityOverride::parse(unquote(value.trim())) {
                    config.rule_overrides.insert(key, severity);
                }
            }
        }
    }

    if config.include.is_empty() {
        config.include.push("**/*.py".to_string());
    }

    Ok(config)
}

fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(':')?;
    Some((key, value))
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        return &value[1..value.len() - 1];
    }
    value
}

#[cfg(test)]
mod tests {
    use super::{load_config, parse_yaml_config, Config};
    use std::fs;

    #[test]
    fn parses_yaml_config() {
        let parsed = parse_yaml_config(
            r#"
lint:
  include:
    - "**/*.py"
  exclude:
    - tests/**
rules:
  owt101: off
  OWP202: error
"#,
        )
        .expect("config should parse");

        assert_eq!(parsed.include, vec!["**/*.py"]);
        assert_eq!(parsed.exclude, vec!["tests/**"]);
        assert!(parsed.rule_overrides.contains_key("OWT101"));
        assert!(parsed.rule_overrides.contains_key("OWP202"));
    }

    #[test]
    fn returns_defaults_without_file() {
        let config = Config::default();
        assert!(config.include.iter().any(|entry| entry == "**/*.py"));
    }

    #[test]
    fn explicit_path_missing_is_error() {
        let path = std::env::temp_dir().join("owui-lint-missing-config.yml");
        let result = load_config(Some(&path));
        assert!(result.is_err());
    }

    #[test]
    fn loads_from_explicit_path() {
        let dir = std::env::temp_dir().join(format!(
            "owui_lint_cfg_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp directory should be created");
        let path = dir.join("cfg.yml");
        fs::write(&path, "rules:\n  OWT101: off\n").expect("config file should be written");

        let cfg = load_config(Some(&path)).expect("config should load");
        assert!(cfg.rule_overrides.contains_key("OWT101"));

        fs::remove_dir_all(dir).expect("temp directory should be removed");
    }
}
