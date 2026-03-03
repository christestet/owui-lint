use owui_lint::analysis::analyze_file;
use std::path::PathBuf;

#[test]
fn multiline_string_bug() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("test_multiline.py");
    let content = r#"class Filter:
    def __init__(self):
        self.doc = """
some text
    def inlet(self, body):
        pass
"""
    def inlet(self, body):
        pass
"#;
    std::fs::write(&path, content).unwrap();

    let info = analyze_file(&path);
    assert_eq!(info.classes.len(), 1);
    let methods = &info.classes[0].methods;
    assert_eq!(methods.len(), 2, "Methods: {:?}", methods);
}
