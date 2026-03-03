use owui_lint::analysis::analyze_file;
use std::path::PathBuf;

#[test]
fn multiline_string_bug() {
    let path = PathBuf::from("/tmp/test_multiline.py");
    let info = analyze_file(&path);
    assert_eq!(info.classes.len(), 1);
    let methods = &info.classes[0].methods;
    assert_eq!(methods.len(), 2, "Methods: {:?}", methods);
}
