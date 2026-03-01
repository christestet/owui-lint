use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{Parser, Subcommand, ValueEnum};

use crate::config::load_config;
use crate::linter::{discover_python_files, lint_files};
use crate::output::{
    format_github, format_json, format_rule_explanation, format_rule_list_json,
    format_rule_list_text, format_sarif, format_text,
};
use crate::rules::is_known_rule;

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Github,
    Sarif,
}

#[derive(Debug, Clone, ValueEnum)]
enum FailOn {
    None,
    Error,
    Warning,
}

#[derive(Debug, Clone, ValueEnum)]
enum RulesFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Parser)]
struct LintArgs {
    #[arg(
        value_name = "TARGET",
        help = "File(s) or directory path(s) to lint. Directories are scanned recursively for *.py."
    )]
    targets: Vec<PathBuf>,

    #[arg(
        short = 'c',
        long,
        value_name = "PATH",
        help = "Path to config file (default lookup: config.yml, owui-lint.yml, owui-lint.yaml)."
    )]
    config: Option<PathBuf>,

    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        help = "Output format for lint findings."
    )]
    format: OutputFormat,

    #[arg(
        short = 'o',
        long,
        value_name = "PATH",
        help = "Write output to a file instead of stdout."
    )]
    output: Option<PathBuf>,

    #[arg(
        long,
        value_enum,
        default_value_t = FailOn::Error,
        help = "Exit behavior: none=always 0, error=non-zero on errors, warning=non-zero on any findings."
    )]
    fail_on: FailOn,
}

#[derive(Debug, Clone, Subcommand)]
enum Commands {
    /// Lint one or more targets.
    Lint(LintArgs),
    /// List all supported lint rules.
    Rules {
        #[arg(long, value_enum, default_value_t = RulesFormat::Text)]
        format: RulesFormat,
        #[arg(short = 'o', long, value_name = "PATH")]
        output: Option<PathBuf>,
    },
    /// Explain a lint rule (for example: OWT101).
    Explain {
        rule_id: String,
        #[arg(short = 'o', long, value_name = "PATH")]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Parser)]
#[command(
    name = "owui-lint",
    about = "Lint Open WebUI extensions (Tools, Pipe, Filter, Action, Pipeline).",
    long_about = "owui-lint validates Open WebUI extension files and reports actionable issues.\n\nUse positional targets directly for linting, or use subcommands for rule discovery.",
    after_help = "Examples:\n  owui-lint extensions/\n  owui-lint lint extensions/ --format json --output lint-report.json\n  owui-lint rules\n  owui-lint explain OWT101"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    lint: LintArgs,
}

pub fn run<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(err) => {
            let exit_code = match err.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
                _ => 2,
            };
            if exit_code == 0 {
                print!("{err}");
            } else {
                eprint!("{err}");
            }
            return exit_code;
        }
    };

    match cli.command {
        Some(Commands::Lint(args)) => run_lint(args),
        Some(Commands::Rules { format, output }) => {
            let rendered = match format {
                RulesFormat::Text => format_rule_list_text(),
                RulesFormat::Json => format_rule_list_json(),
            };
            write_or_print(&rendered, output)
        }
        Some(Commands::Explain { rule_id, output }) => {
            let normalized = rule_id.trim().to_ascii_uppercase();
            let rendered = match format_rule_explanation(&normalized) {
                Some(content) => content,
                None => {
                    eprintln!(
                        "Unknown rule '{normalized}'. Run `owui-lint rules` to see available rule IDs."
                    );
                    return 2;
                }
            };
            write_or_print(&rendered, output)
        }
        None => run_lint(cli.lint),
    }
}

fn run_lint(cli: LintArgs) -> i32 {
    if cli.targets.is_empty() {
        eprintln!("No targets provided. Use `owui-lint <target>` or `owui-lint lint <target>`.");
        return 2;
    }

    let config = match load_config(cli.config.as_deref()) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return 2;
        }
    };
    let unknown_overrides = config
        .rule_overrides
        .keys()
        .filter(|rule_id| !is_known_rule(rule_id))
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_overrides.is_empty() {
        eprintln!(
            "Warning: unknown rule override(s) in config: {}. Run `owui-lint rules` to list valid IDs.",
            unknown_overrides.join(", ")
        );
    }

    let files = match discover_python_files(&cli.targets, &config.include, &config.exclude) {
        Ok(files) => files,
        Err(err) => {
            eprintln!("{err}");
            return 2;
        }
    };

    if files.is_empty() {
        eprintln!(
            "No Python files matched the given targets/patterns. Check your target paths and include/exclude rules."
        );
        return 2;
    }

    let (issues, summary) = lint_files(&files, &config);

    let output = match cli.format {
        OutputFormat::Text => format_text(&issues, &summary),
        OutputFormat::Json => format_json(&issues, &summary),
        OutputFormat::Github => format_github(&issues, &summary),
        OutputFormat::Sarif => format_sarif(&issues, &summary, env!("CARGO_PKG_VERSION")),
    };

    let write_code = write_or_print(&output, cli.output);
    if write_code != 0 {
        return write_code;
    }

    match cli.fail_on {
        FailOn::None => 0,
        FailOn::Warning => usize::from(summary.errors > 0 || summary.warnings > 0) as i32,
        FailOn::Error => usize::from(summary.errors > 0) as i32,
    }
}

fn write_or_print(output: &str, output_path: Option<PathBuf>) -> i32 {
    if let Some(path) = output_path {
        if let Err(err) = fs::write(&path, format!("{output}\n")) {
            eprintln!("Failed to write {}: {err}", path.display());
            return 2;
        }
    } else {
        println!("{output}");
    }

    0
}
