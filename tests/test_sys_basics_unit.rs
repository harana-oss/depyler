
// Module: sys - Basic system functions
// Status: GREEN phase - Tests enabled

use crate::test_helpers::transpile_and_check;

// 
#[test]
fn test_sys_argv() {
    let python = r#"
import sys

def get_args() -> list:
    return sys.argv
"#;

    let result = transpile_and_check(python, &[]);

    // Should get command line arguments
    assert!(result.contains("args") && result.contains("collect"));
}

// 
#[test]
fn test_sys_exit() {
    let python = r#"
import sys

def exit_program(code: int) -> None:
    sys.exit(code)
"#;

    let result = transpile_and_check(python, &[]);

    // Should exit with code
    assert!(result.contains("std::process::exit"));
}

// 
#[test]
fn test_sys_platform() {
    let python = r#"
import sys

def get_platform() -> str:
    return sys.platform
"#;

    let result = transpile_and_check(python, &[]);

    // Should get platform name
    assert!(result.contains("linux") || result.contains("darwin") || result.contains("win32"));
}

// Total: 3 sys module functions implemented
// Coverage: sys.argv, sys.exit(), sys.platform
