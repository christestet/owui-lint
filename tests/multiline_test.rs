use owui_lint::analysis::analyze_file;

#[test]
fn multiline_string_bug() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
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
    std::fs::write(&path, content).expect("fixture should be written");

    let info = analyze_file(&path);
    assert_eq!(info.classes.len(), 1);
    let methods = &info.classes[0].methods;
    assert_eq!(methods.len(), 2, "Methods: {:?}", methods);
    let inlet = methods
        .iter()
        .find(|method| method.name == "inlet")
        .expect("inlet should be detected");
    assert_eq!(
        inlet.line, 8,
        "inlet should be from real method, not string"
    );
}
