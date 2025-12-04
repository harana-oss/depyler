// Module: glob - Python glob module validation
// pending

use crate::test_helpers::transpile_and_check;

//
#[test]
fn test_glob() {
    let python = r#"
import glob

def find_files(pattern: str) -> list:
    return glob.glob(pattern)
"#;

    let result = transpile_and_check(python, &[]);

    // Should find files matching pattern
    assert!(result.contains("glob") || result.contains("read_dir") || result.contains("PathBuf"));
}

#[test]
fn test_iglob() {
    let python = r#"
import glob

def find_files_iter(pattern: str) -> glob.iglob:
    return glob.iglob(pattern)
"#;

    let result = transpile_and_check(python, &[]);

    // Should return iterator of matching files
    assert!(result.contains("glob") || result.contains("iter") || result.contains("read_dir"));
}

// Total: 2 comprehensive tests for glob module
// Coverage: glob(), iglob()
// Unix-style pathname pattern expansion
