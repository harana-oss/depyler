use crate::test_helpers;
use crate::test_helpers::transpile;
use std::process::Command;

#[test]
#[allow(non_snake_case)]
fn test_for_loop_comparison_compiles() {
    let python_code = r#"
def find_min(numbers: list[int]) -> int:
    """Find minimum value in a list."""
    if not numbers:
        return 0
    min_val = numbers[0]
    for num in numbers:
        if num < min_val:
            min_val = num
    return min_val
"#;

    let rust_code = transpile(python_code);

    // Write to temp file
    let temp_file = "/tmp/test_comparison.rs";
    std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

    // Attempt to compile with rustc
    let output = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg(temp_file)
        .arg("-o")
        .arg("/tmp/test_comparison.rlib")
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        eprintln!("\n=== rustc stderr ===");
        eprintln!("{}", stderr);

        assert!(
            !stderr.contains("mismatched types"),
            "Type mismatch in for loop comparison!\n\
             Expected: Iterator dereferencing handled automatically\n\
             Actual: Generated code has &T vs T type mismatch\n\
             \n\
             Common error patterns:\n\
             - 'expected `&i32`, found `i32`' (comparing &T with T)\n\
             - 'expected `i32`, found `&i32`' (assigning &T to T)\n\
             \n\
             Generated Rust code:\n{}\n\
             \n\
             rustc error:\n{}",
            rust_code,
            stderr
        );
    }

    assert!(
        output.status.success(),
        "Compilation should succeed\n\
         Generated code:\n{}\n\
         Errors:\n{}",
        rust_code,
        stderr
    );
}

#[test]
#[allow(non_snake_case)]
fn test_for_loop_arithmetic_compiles() {
    let python_code = r#"
def sum_list(numbers: list[int]) -> int:
    """Sum all numbers in a list."""
    total = 0
    for num in numbers:
        total = total + num
    return total
"#;

    let rust_code = transpile(python_code);

    // Write to temp file
    let temp_file = "/tmp/test_arithmetic.rs";
    std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

    // Attempt to compile
    let output = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg(temp_file)
        .arg("-o")
        .arg("/tmp/test_arithmetic.rlib")
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        eprintln!("\n=== rustc stderr (arithmetic) ===");
        eprintln!("{}", stderr);

        assert!(
            !stderr.contains("mismatched types"),
            "Type mismatch in for loop arithmetic!\n\
             Error: {}\n\
             Code: {}",
            stderr,
            rust_code
        );
    }

    assert!(
        output.status.success(),
        "Arithmetic in for loop should compile\n\
         Code: {}\nErrors: {}",
        rust_code,
        stderr
    );
}

#[test]
#[allow(non_snake_case)]
fn test_for_loop_assignment_compiles() {
    let python_code = r#"
def find_max(numbers: list[int]) -> int:
    """Find maximum value in a list."""
    if not numbers:
        return 0
    max_val = numbers[0]
    for num in numbers:
        if num > max_val:
            max_val = num
    return max_val
"#;

    let rust_code = transpile(python_code);

    // Write to temp file
    let temp_file = "/tmp/test_assignment.rs";
    std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

    // Attempt to compile
    let output = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg(temp_file)
        .arg("-o")
        .arg("/tmp/test_assignment.rlib")
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        eprintln!("\n=== rustc stderr (assignment) ===");
        eprintln!("{}", stderr);

        assert!(
            !stderr.contains("mismatched types"),
            "Type mismatch in for loop assignment!\n\
             Error: {}\n\
             Code: {}",
            stderr,
            rust_code
        );
    }

    assert!(
        output.status.success(),
        "Assignment in for loop should compile\n\
         Code: {}\nErrors: {}",
        rust_code,
        stderr
    );
}

#[test]
#[allow(non_snake_case)]
fn test_for_loop_string_compiles() {
    let python_code = r#"
def find_longest(words: list[str]) -> str:
    """Find the longest word in a list."""
    if not words:
        return ""
    longest = words[0]
    for word in words:
        if len(word) > len(longest):
            longest = word
    return longest
"#;

    let rust_code = transpile(python_code);

    // Write to temp file
    let temp_file = "/tmp/test_string.rs";
    std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

    // Attempt to compile
    let output = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg(temp_file)
        .arg("-o")
        .arg("/tmp/test_string.rlib")
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        eprintln!("\n=== rustc stderr (string) ===");
        eprintln!("{}", stderr);

        assert!(
            !stderr.contains("mismatched types"),
            "Type mismatch in for loop over strings!\n\
             Error: {}\n\
             Code: {}",
            stderr,
            rust_code
        );
    }

    assert!(
        output.status.success(),
        "For loop over strings should compile\n\
         Code: {}\nErrors: {}",
        rust_code,
        stderr
    );
}
