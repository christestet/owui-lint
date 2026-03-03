mod parsing;
mod syntax;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::models::{ClassInfo, FunctionInfo, ModuleInfo, NestedClassInfo, SyntaxErrorInfo};
use crate::util::count_indent;

use parsing::{
    collect_function_signature, extract_module_docstring, is_docstring_line, is_function_start,
    parse_class_definition, parse_function_definition, parse_import, parse_import_from,
    parse_valve_fields, returns_body, self_assignment_name,
};
use syntax::detect_syntax_error;

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

fn update_multiline_state(raw_line: &str, in_multiline: &mut Option<&'static str>) {
    let mut chars = raw_line.chars();
    let mut escaped = false;
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }

        let is_triple = |c: char, mut it: std::str::Chars| -> bool {
            it.next() == Some(c) && it.next() == Some(c)
        };

        if let Some(quote) = *in_multiline {
            let quote_char = quote.chars().next().unwrap();
            if ch == quote_char && is_triple(quote_char, chars.clone()) {
                *in_multiline = None;
                chars.next();
                chars.next();
                continue;
            }
        } else {
            if !in_single && !in_double {
                if ch == '"' && is_triple('"', chars.clone()) {
                    *in_multiline = Some("\"\"\"");
                    chars.next();
                    chars.next();
                    continue;
                } else if ch == '\'' && is_triple('\'', chars.clone()) {
                    *in_multiline = Some("'''");
                    chars.next();
                    chars.next();
                    continue;
                } else if ch == '#' {
                    break;
                }
            }
            if ch == '\'' && !in_double {
                in_single = !in_single;
            } else if ch == '"' && !in_single {
                in_double = !in_double;
            }
        }
    }
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
    let mut in_multiline: Option<&'static str> = None;

    while line_idx < lines.len() {
        let raw_line = lines[line_idx];
        let line_no = line_idx + 1;
        let indent = count_indent(raw_line);
        let trimmed = raw_line.trim();

        let was_in_multiline = in_multiline.is_some();
        update_multiline_state(raw_line, &mut in_multiline);

        if trimmed.is_empty() || trimmed.starts_with('#') {
            line_idx += 1;
            continue;
        }

        if was_in_multiline {
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

    extract_valve_fields_for_classes(&mut module, &lines);

    module
}

fn extract_valve_fields_for_classes(module: &mut ModuleInfo, lines: &[&str]) {
    for class in &mut module.classes {
        for nested in &mut class.inner_classes {
            if nested.name == "Valves" || nested.name == "UserValves" {
                let class_line_idx = nested.line.saturating_sub(1);
                nested.fields = parse_valve_fields(lines, class_line_idx);
            }
        }
    }
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
            .push(NestedClassInfo {
                name,
                bases,
                line: line_no,
                fields: Vec::new(),
            });
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
        let is_init_method = definition.name == "__init__";
        module.classes[class_index].methods.push(FunctionInfo {
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
        let method_index = module.classes[class_index].methods.len() - 1;
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
