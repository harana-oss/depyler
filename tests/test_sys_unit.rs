
// Module: sys - Python sys module validation

use crate::test_helpers::transpile_and_check;

// 
#[test]
fn test_exit() {
    let python = r#"
import sys

def terminate(code: int) -> None:
    sys.exit(code)
"#;

    let result = transpile_and_check(python, &[]);

    // Should exit with code
    assert!(result.contains("process::exit"));
}

// 
#[test]
fn test_stdout_write() {
    let python = r#"
import sys

def write_stdout(message: str) -> None:
    sys.stdout.write(message)
"#;

    let result = transpile_and_check(python, &[]);

    // Should write to stdout
    assert!(result.contains("stdout") || result.contains("print"));
}

#[test]
fn test_stderr_write() {
    let python = r#"
import sys

def write_stderr(message: str) -> None:
    sys.stderr.write(message)
"#;

    let result = transpile_and_check(python, &[]);

    // Should write to stderr
    assert!(result.contains("stderr") || result.contains("eprintln"));
}

// Total: 3 tests for sys module
// Coverage: exit(), stdout.write(), stderr.write()
