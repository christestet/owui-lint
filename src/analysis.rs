use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::models::{ClassInfo, FunctionInfo, ModuleInfo, NestedClassInfo, SyntaxErrorInfo};
use crate::util::{count_indent, strip_inline_comment};

#[derive(Debug)]
struct ClassContext {
    indent: usize,
    class_index: Option<usize>,
    first_stmt_seen: bool,
}

#[derive(Debug)]
enum FunctionTarget {
    Module(usize),
    Method {
        class_index: usize,
        method_index: usize,
    },
    Ignore,
}

#[derive(Debug)]
struct FunctionContext {
    indent: usize,
    target: FunctionTarget,
    class_index: Option<usize>,
    is_init_method: bool,
    first_stmt_seen: bool,
}

#[derive(Debug)]
enum Context {
    Class(ClassContext),
    Function(FunctionContext),
}

pub fn analyze_file(path: &Path) -> ModuleInfo {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            return ModuleInfo {
                path: path.to_path_buf(),
                syntax_ok: false,
                syntax_error: Some(SyntaxErrorInfo {
                    message: format!("Unable to read file: {err}"),
                    line: 1,
                    column: 1,
                    text: None,
                }),
                module_docstring: None,
                module_docstring_line: None,
                imports: Vec::new(),
                functions: Vec::new(),
                classes: Vec::new(),
            };
        }
    };

    let syntax_error = detect_syntax_error(&source);
    if let Some(error) = syntax_error {
        return ModuleInfo {
            path: path.to_path_buf(),
            syntax_ok: false,
            syntax_error: Some(error),
            module_docstring: None,
            module_docstring_line: None,
            imports: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
        };
    }

    parse_module(path, &source)
}

fn parse_module(path: &Path, source: &str) -> ModuleInfo {
    let (module_docstring, module_docstring_line) = match extract_module_docstring(source) {
        Some((ds, ln)) => (Some(ds), Some(ln)),
        None => (None, None),
    };

    let mut module = ModuleInfo {
        path: path.to_path_buf(),
        syntax_ok: true,
        syntax_error: None,
        module_docstring,
        module_docstring_line,
        imports: Vec::new(),
        functions: Vec::new(),
        classes: Vec::new(),
    };

    let mut contexts: Vec<Context> = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut line_idx = 0;

    while line_idx < lines.len() {
        let raw_line = lines[line_idx];
        let line_no = line_idx + 1;
        let indent = count_indent(raw_line);
        let trimmed = raw_line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            line_idx += 1;
            continue;
        }

        while let Some(context) = contexts.last() {
            let should_pop = match context {
                Context::Class(class_ctx) => indent <= class_ctx.indent,
                Context::Function(function_ctx) => indent <= function_ctx.indent,
            };
            if should_pop {
                contexts.pop();
            } else {
                break;
            }
        }

        if let Some(current) = contexts.last_mut() {
            match current {
                Context::Class(class_ctx) => {
                    if indent > class_ctx.indent && !class_ctx.first_stmt_seen {
                        if let Some(class_index) = class_ctx.class_index {
                            module.classes[class_index].has_docstring = is_docstring_line(trimmed);
                        }
                        class_ctx.first_stmt_seen = true;
                    }
                }
                Context::Function(function_ctx) => {
                    if indent > function_ctx.indent {
                        if !function_ctx.first_stmt_seen {
                            set_function_docstring(&mut module, &function_ctx.target, trimmed);
                            function_ctx.first_stmt_seen = true;
                        }

                        set_function_returns_body(&mut module, &function_ctx.target, trimmed);

                        if function_ctx.is_init_method
                            && let Some(class_index) = function_ctx.class_index
                            && let Some(name) = self_assignment_name(trimmed)
                        {
                            module.classes[class_index]
                                .init_assignments
                                .insert(name.to_string());
                        }
                    }
                }
            }
        }

        if indent == 0 {
            if let Some(import) = parse_import(trimmed) {
                module.imports.push(import);
                line_idx += 1;
                continue;
            }
            if let Some(import_from) = parse_import_from(trimmed) {
                module.imports.push(import_from);
                line_idx += 1;
                continue;
            }
        }

        if handle_class_definition(&mut module, &mut contexts, trimmed, indent, line_no) {
            line_idx += 1;
            continue;
        }

        if is_function_start(trimmed) {
            let consumed = handle_function_definition(
                &mut module,
                &mut contexts,
                &lines,
                line_idx,
                indent,
                line_no,
            );
            line_idx += consumed;
            continue;
        }

        line_idx += 1;
    }

    module
}

fn handle_class_definition(
    module: &mut ModuleInfo,
    contexts: &mut Vec<Context>,
    trimmed: &str,
    indent: usize,
    line_no: usize,
) -> bool {
    let Some((name, bases)) = parse_class_definition(trimmed) else {
        return false;
    };
    let column = indent + 1;
    if let Some(parent_index) = current_top_level_class_index(contexts) {
        module.classes[parent_index]
            .inner_classes
            .push(NestedClassInfo { name, bases });
        contexts.push(Context::Class(ClassContext {
            indent,
            class_index: None,
            first_stmt_seen: false,
        }));
    } else if indent == 0 {
        module.classes.push(ClassInfo {
            name,
            line: line_no,
            column,
            bases,
            methods: Vec::new(),
            inner_classes: Vec::new(),
            init_assignments: BTreeSet::new(),
            has_docstring: false,
        });
        let class_index = module.classes.len() - 1;
        contexts.push(Context::Class(ClassContext {
            indent,
            class_index: Some(class_index),
            first_stmt_seen: false,
        }));
    } else {
        contexts.push(Context::Class(ClassContext {
            indent,
            class_index: None,
            first_stmt_seen: false,
        }));
    }
    true
}

fn handle_function_definition(
    module: &mut ModuleInfo,
    contexts: &mut Vec<Context>,
    lines: &[&str],
    line_idx: usize,
    indent: usize,
    line_no: usize,
) -> usize {
    let (signature, consumed_lines) = collect_function_signature(lines, line_idx);
    let signature = signature.trim();
    let Some(definition) = parse_function_definition(signature) else {
        return consumed_lines;
    };
    let column = indent + 1;
    if let Some(class_index) = direct_parent_class_index(contexts) {
        module.classes[class_index].methods.push(FunctionInfo {
            name: definition.name.clone(),
            line: line_no,
            column,
            args: definition.args,
            decorators: Vec::new(),
            is_async: definition.is_async,
            has_docstring: false,
            returns_annotation: definition.returns_annotation,
            returns_body: false,
        });
        let method_index = module.classes[class_index].methods.len() - 1;
        let is_init_method = definition.name == "__init__";
        contexts.push(Context::Function(FunctionContext {
            indent,
            target: FunctionTarget::Method {
                class_index,
                method_index,
            },
            class_index: Some(class_index),
            is_init_method,
            first_stmt_seen: false,
        }));
    } else if indent == 0 {
        module.functions.push(FunctionInfo {
            name: definition.name,
            line: line_no,
            column,
            args: definition.args,
            decorators: Vec::new(),
            is_async: definition.is_async,
            has_docstring: false,
            returns_annotation: definition.returns_annotation,
            returns_body: false,
        });
        let function_index = module.functions.len() - 1;
        contexts.push(Context::Function(FunctionContext {
            indent,
            target: FunctionTarget::Module(function_index),
            class_index: None,
            is_init_method: false,
            first_stmt_seen: false,
        }));
    } else {
        contexts.push(Context::Function(FunctionContext {
            indent,
            target: FunctionTarget::Ignore,
            class_index: current_top_level_class_index(contexts),
            is_init_method: false,
            first_stmt_seen: false,
        }));
    }
    consumed_lines
}

fn is_function_start(trimmed: &str) -> bool {
    trimmed.starts_with("def ") || trimmed.starts_with("async def ")
}

fn collect_function_signature(lines: &[&str], start_idx: usize) -> (String, usize) {
    let mut signature = strip_inline_comment(lines[start_idx].trim())
        .trim()
        .to_string();
    let mut consumed = 1;

    while !function_signature_complete(&signature) && start_idx + consumed < lines.len() {
        let next = strip_inline_comment(lines[start_idx + consumed].trim())
            .trim()
            .to_string();
        if !next.is_empty() {
            if !signature.is_empty() {
                signature.push(' ');
            }
            signature.push_str(&next);
        }
        consumed += 1;
    }

    (signature, consumed)
}

fn function_signature_complete(signature: &str) -> bool {
    let trimmed = signature.trim_end();
    if !trimmed.ends_with(':') {
        return false;
    }

    paren_balance(trimmed) == 0
}

fn paren_balance(value: &str) -> i32 {
    let mut balance = 0_i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        if in_double {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }

        match ch {
            '\'' => in_single = true,
            '"' => in_double = true,
            '(' => balance += 1,
            ')' => balance -= 1,
            _ => {}
        }
    }

    balance
}

fn direct_parent_class_index(contexts: &[Context]) -> Option<usize> {
    match contexts.last() {
        Some(Context::Class(class_ctx)) => class_ctx.class_index,
        _ => None,
    }
}

fn current_top_level_class_index(contexts: &[Context]) -> Option<usize> {
    for context in contexts.iter().rev() {
        match context {
            Context::Class(class_ctx) => {
                if let Some(class_index) = class_ctx.class_index {
                    return Some(class_index);
                }
            }
            Context::Function(function_ctx) => {
                if let Some(class_index) = function_ctx.class_index {
                    return Some(class_index);
                }
            }
        }
    }
    None
}

fn set_function_docstring(module: &mut ModuleInfo, target: &FunctionTarget, trimmed: &str) {
    let has_docstring = is_docstring_line(trimmed);
    match target {
        FunctionTarget::Module(index) => module.functions[*index].has_docstring = has_docstring,
        FunctionTarget::Method {
            class_index,
            method_index,
        } => module.classes[*class_index].methods[*method_index].has_docstring = has_docstring,
        FunctionTarget::Ignore => {}
    }
}

fn set_function_returns_body(module: &mut ModuleInfo, target: &FunctionTarget, trimmed: &str) {
    if !returns_body(trimmed) {
        return;
    }
    match target {
        FunctionTarget::Module(index) => module.functions[*index].returns_body = true,
        FunctionTarget::Method {
            class_index,
            method_index,
        } => module.classes[*class_index].methods[*method_index].returns_body = true,
        FunctionTarget::Ignore => {}
    }
}

fn returns_body(trimmed: &str) -> bool {
    if let Some(rest) = trimmed.strip_prefix("return") {
        return strip_inline_comment(rest).trim() == "body";
    }
    false
}

fn self_assignment_name(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("self.")?;
    let name_end = rest
        .chars()
        .position(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'))?;
    let name = &rest[..name_end];
    let tail = rest[name_end..].trim_start();
    if tail.starts_with('=') || tail.starts_with(':') {
        return Some(name);
    }
    None
}

fn parse_import(trimmed: &str) -> Option<String> {
    let payload = trimmed.strip_prefix("import ")?;
    let first = payload.split(',').next()?.trim();
    if first.is_empty() {
        return None;
    }
    Some(first.split_whitespace().next().unwrap_or(first).to_string())
}

fn parse_import_from(trimmed: &str) -> Option<String> {
    let payload = trimmed.strip_prefix("from ")?;
    let module_name = payload.split_whitespace().next()?;
    if module_name.is_empty() {
        return None;
    }
    Some(module_name.to_string())
}

#[derive(Debug)]
struct FunctionDef {
    name: String,
    args: Vec<String>,
    is_async: bool,
    returns_annotation: bool,
}

fn parse_function_definition(trimmed: &str) -> Option<FunctionDef> {
    let (payload, is_async) = if let Some(payload) = trimmed.strip_prefix("async def ") {
        (payload, true)
    } else if let Some(payload) = trimmed.strip_prefix("def ") {
        (payload, false)
    } else {
        return None;
    };

    let open_paren = payload.find('(')?;
    let name = payload[..open_paren].trim();
    if !is_valid_identifier(name) {
        return None;
    }

    let closing_paren = payload.rfind(')')?;
    if closing_paren <= open_paren {
        return None;
    }

    let args_raw = &payload[open_paren + 1..closing_paren];
    let args = parse_args(args_raw);

    let tail = payload[closing_paren + 1..].trim();
    let returns_annotation = tail.starts_with("->");

    Some(FunctionDef {
        name: name.to_string(),
        args,
        is_async,
        returns_annotation,
    })
}

fn parse_args(raw: &str) -> Vec<String> {
    let mut args = Vec::new();
    for part in raw.split(',') {
        let token = part.trim();
        if token.is_empty() {
            continue;
        }
        let without_default = token.split('=').next().unwrap_or(token).trim();
        let without_type = without_default
            .split(':')
            .next()
            .unwrap_or(without_default)
            .trim();
        let normalized = without_type.trim_start_matches('*').trim();
        if is_valid_identifier(normalized) {
            args.push(normalized.to_string());
        }
    }

    args
}

fn parse_class_definition(trimmed: &str) -> Option<(String, Vec<String>)> {
    let payload = trimmed.strip_prefix("class ")?;
    let class_name_end = payload
        .chars()
        .position(|ch| ch == '(' || ch == ':' || ch.is_whitespace())
        .unwrap_or(payload.len());
    let class_name = payload[..class_name_end].trim();
    if !is_valid_identifier(class_name) {
        return None;
    }

    let bases = if let (Some(open), Some(close)) = (payload.find('('), payload.rfind(')')) {
        if close > open {
            payload[open + 1..close]
                .split(',')
                .map(str::trim)
                .filter(|base| !base.is_empty())
                .map(|base| base.to_string())
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Some((class_name.to_string(), bases))
}

fn is_valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn extract_module_docstring(source: &str) -> Option<(String, usize)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    // Skip leading blank lines and comment lines.
    while i < lines.len() {
        let t = lines[i].trim();
        if !t.is_empty() && !t.starts_with('#') {
            break;
        }
        i += 1;
    }

    if i >= lines.len() {
        return None;
    }

    let first = lines[i].trim();
    let quote = if first.starts_with("\"\"\"") {
        "\"\"\""
    } else if first.starts_with("'''") {
        "'''"
    } else {
        return None;
    };

    let start_line = i + 1; // 1-indexed
    let after_open = &first[quote.len()..];

    // Single-line docstring closed on the same line.
    if let Some(close_pos) = after_open.find(quote) {
        return Some((after_open[..close_pos].to_string(), start_line));
    }

    // Multi-line: accumulate until closing delimiter.
    let mut content = after_open.to_string();
    i += 1;
    while i < lines.len() {
        let line = lines[i];
        if let Some(close_pos) = line.find(quote) {
            content.push('\n');
            content.push_str(line[..close_pos].trim_end());
            return Some((content, start_line));
        }
        content.push('\n');
        content.push_str(line);
        i += 1;
    }

    None
}

fn is_docstring_line(trimmed: &str) -> bool {
    trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''")
}

struct StringState {
    quote: Option<char>,
    triple_quote: bool,
    escaped: bool,
}

fn advance_through_string(
    state: &mut StringState,
    chars: &[char],
    idx: usize,
    column: &mut usize,
) -> usize {
    let active_quote = match state.quote {
        Some(q) => q,
        None => return idx,
    };
    let ch = chars[idx];

    if state.escaped {
        state.escaped = false;
        return idx + 1;
    }

    if ch == '\\' {
        state.escaped = true;
        return idx + 1;
    }

    if state.triple_quote {
        if ch == active_quote
            && idx + 2 < chars.len()
            && chars[idx + 1] == active_quote
            && chars[idx + 2] == active_quote
        {
            state.quote = None;
            state.triple_quote = false;
            *column += 2;
            return idx + 3;
        }
        return idx + 1;
    }

    if ch == active_quote {
        state.quote = None;
    }
    idx + 1
}

fn check_closing_bracket(
    ch: char,
    stack: &mut Vec<(char, usize, usize)>,
    source: &str,
    line: usize,
    column: usize,
) -> Option<SyntaxErrorInfo> {
    let expected_open = match ch {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        _ => unreachable!(),
    };
    if let Some((open, _, _)) = stack.pop() {
        if open != expected_open {
            return Some(SyntaxErrorInfo {
                message: format!("Mismatched delimiter: found '{ch}'"),
                line,
                column,
                text: source
                    .lines()
                    .nth(line - 1)
                    .map(|line_text| line_text.to_string()),
            });
        }
    } else {
        return Some(SyntaxErrorInfo {
            message: format!("Unmatched closing delimiter: '{ch}'"),
            line,
            column,
            text: source
                .lines()
                .nth(line - 1)
                .map(|line_text| line_text.to_string()),
        });
    }
    None
}

fn detect_syntax_error(source: &str) -> Option<SyntaxErrorInfo> {
    let mut stack: Vec<(char, usize, usize)> = Vec::new();
    let mut line = 1;
    let mut column = 0;
    let chars: Vec<char> = source.chars().collect();
    let mut idx = 0;

    let mut string_state = StringState {
        quote: None,
        triple_quote: false,
        escaped: false,
    };

    while idx < chars.len() {
        let ch = chars[idx];
        if ch == '\n' {
            line += 1;
            column = 0;
            string_state.escaped = false;
            idx += 1;
            continue;
        }

        column += 1;

        if string_state.quote.is_some() {
            idx = advance_through_string(&mut string_state, &chars, idx, &mut column);
            continue;
        }

        if ch == '#' {
            while idx < chars.len() && chars[idx] != '\n' {
                idx += 1;
            }
            continue;
        }

        if ch == '\'' || ch == '"' {
            let is_triple = idx + 2 < chars.len() && chars[idx + 1] == ch && chars[idx + 2] == ch;
            string_state.quote = Some(ch);
            string_state.triple_quote = is_triple;
            if is_triple {
                idx += 3;
                column += 2;
            } else {
                idx += 1;
            }
            continue;
        }

        if matches!(ch, '(' | '[' | '{') {
            stack.push((ch, line, column));
            idx += 1;
            continue;
        }

        if matches!(ch, ')' | ']' | '}') {
            if let Some(error) = check_closing_bracket(ch, &mut stack, source, line, column) {
                return Some(error);
            }
            idx += 1;
            continue;
        }

        idx += 1;
    }

    if let Some((open, open_line, open_col)) = stack.pop() {
        return Some(SyntaxErrorInfo {
            message: format!("Unclosed delimiter: '{open}'"),
            line: open_line,
            column: open_col,
            text: source
                .lines()
                .nth(open_line.saturating_sub(1))
                .map(|line_text| line_text.to_string()),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::analyze_file;

    #[test]
    fn syntax_error_is_detected() {
        let dir = test_dir("syntax");
        let path = dir.join("broken.py");
        fs::write(
            &path,
            "class Pipe:\n    def pipe(self, body):\n        return body\n    def x(\n",
        )
        .expect("test file should be written");

        let module = analyze_file(&path);
        assert!(!module.syntax_ok);
        assert!(module.syntax_error.is_some());

        fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[test]
    fn methods_and_inner_classes_are_collected() {
        let dir = test_dir("parse");
        let path = dir.join("tools.py");
        fs::write(
            &path,
            "class Tools:\n    class Valves(BaseModel):\n        pass\n\n    def __init__(self):\n        self.valves = self.Valves()\n\n    async def get_weather(self, city: str) -> str:\n        \"\"\"Fetch weather\"\"\"\n        return city\n",
        )
        .expect("test file should be written");

        let module = analyze_file(&path);
        assert!(module.syntax_ok);
        let tools = module
            .classes
            .iter()
            .find(|item| item.name == "Tools")
            .expect("Tools class should be found");
        assert!(tools.inner_class("Valves").is_some());
        assert!(tools.init_assignments.contains("valves"));
        let method = tools
            .method("get_weather")
            .expect("method should be collected");
        assert!(method.has_docstring);

        fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[test]
    fn multiline_method_signature_is_collected() {
        let dir = test_dir("multiline_signature");
        let path = dir.join("pipe.py");
        fs::write(
            &path,
            "class Pipe:\n    async def pipe(\n        self,\n        body: dict,\n    ) -> dict:\n        return body\n",
        )
        .expect("test file should be written");

        let module = analyze_file(&path);
        assert!(module.syntax_ok);
        let pipe = module
            .classes
            .iter()
            .find(|item| item.name == "Pipe")
            .expect("Pipe class should be found");
        assert!(pipe.method("pipe").is_some());

        fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    fn test_dir(prefix: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "owui_lint_analysis_{prefix}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("test directory should be created");
        path
    }
}
