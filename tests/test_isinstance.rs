// isinstance transpilation tests

use crate::test_helpers::transpile;
use std::process::Command;

#[test]
#[allow(non_snake_case)]
fn test_isinstance_int_removed() {
    let python = r#"
def check_int(x: int) -> bool:
    return isinstance(x, int)
"#;

    let result: Result<String, String> = Ok(transpile(python));

    assert!(result.is_ok(), "Transpilation should succeed");
    let rust = result.unwrap();

    eprintln!("=== Generated Rust code ===");
    eprintln!("{}", rust);

    assert!(
        !rust.contains("isinstance"),
        "Expected: true, Actual: isinstance(x, int). Generated code: {}",
        rust
    );

    assert!(
        rust.contains("true"),
        "Type system guarantees that x: i32 is always int. Generated code: {}",
        rust
    );
}

#[test]
#[allow(non_snake_case)]
fn test_isinstance_str_removed() {
    let python = r#"
def check_str(s: str) -> bool:
    return isinstance(s, str)
"#;

    let rust = transpile(python);

    assert!(!rust.contains("isinstance"), "isinstance should be removed");
    assert!(rust.contains("true"), "Should return true");
}

#[test]
#[allow(non_snake_case)]
fn test_isinstance_compiles() {
    let python = r#"
def check_type(value: int) -> bool:
    return isinstance(value, int)
"#;

    let rust = transpile(python);

    let temp_file = "/tmp/test_depyler_0269.rs";
    std::fs::write(temp_file, &rust).expect("Failed to write temp file");

    let output = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg(temp_file)
        .arg("-o")
        .arg("/tmp/test_depyler_0269.rlib")
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        eprintln!("=== Compilation errors ===");
        eprintln!("{}", stderr);
    }

    assert!(
        output.status.success(),
        "Code should compile. Errors: {}. Generated code: {}",
        stderr,
        rust
    );
}

#[test]
#[allow(non_snake_case)]
fn test_isinstance_multiple_types() {
    let python = r#"
def check_types(x: int, s: str) -> bool:
    return isinstance(x, int) and isinstance(s, str)
"#;

    let rust = transpile(python);

    assert!(!rust.contains("isinstance"), "All isinstance calls should be removed");

    let true_count = rust.matches("true").count();
    assert!(
        true_count >= 2,
        "Should have at least 2 'true' values. Found: {}. Generated code: {}",
        true_count,
        rust
    );
}

#[test]
#[allow(non_snake_case)]
fn test_isinstance_in_if_statement() {
    let python = r#"
def validate(data: str) -> bool:
    if isinstance(data, str):
        return True
    return False
"#;

    let rust = transpile(python);

    assert!(!rust.contains("isinstance"), "isinstance should be removed");
    assert!(rust.contains("true"), "Should contain true");

    let temp_file = "/tmp/test_if.rs";
    std::fs::write(temp_file, &rust).unwrap();

    let output = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", temp_file])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Code with isinstance in if statement should compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[allow(non_snake_case)]
fn test_isinstance_with_float() {
    let python = r#"
def check_float(x: float) -> bool:
    return isinstance(x, float)
"#;

    let rust = transpile(python);

    assert!(!rust.contains("isinstance"), "isinstance should be removed");
    assert!(rust.contains("true"), "Should return true");
}

#[test]
#[allow(non_snake_case)]
fn test_isinstance_with_bool() {
    let python = r#"
def check_bool(b: bool) -> bool:
    return isinstance(b, bool)
"#;

    let rust = transpile(python);

    assert!(!rust.contains("isinstance"), "isinstance should be removed");
    assert!(rust.contains("true"), "Should return true");
}
