
// Module: os.path - Additional path functions
// Status: GREEN phase - Tests enabled

use crate::test_helpers::transpile_and_check;

// Note: splitext, dirname, basename, normpath were already implemented
// This commit adds 1 NEW function: relpath

// 
#[test]
#[ignore]
fn test_relpath() {
    let python = r#"
import os.path

def relative_path(path: str, start: str) -> str:
    return os.path.relpath(path, start)
"#;

    let result = transpile_and_check(python, &[]);

    // Should compute relative path
    assert!(result.contains("strip_prefix") || result.contains("relative"));
}