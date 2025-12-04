//! Consolidated tests for string operations
//!
//! Tests cover:
//! - String methods (lstrip, rstrip, isalnum, count)
//! - String slicing
//! - F-strings

mod test_helpers;
use test_helpers::transpile_and_check;

use depyler_core::DepylerPipeline;

// ============================================================================
// String Methods Tests
// ============================================================================

mod string_methods {
    use super::*;

    #[test]
    fn test_lstrip_basic() {
        let python_code = r#"
def strip_leading(s: str) -> str:
    return s.lstrip()
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains("trim_start()"), "Should contain trim_start()");
        assert!(!rust_code.contains("lstrip()"), "Should not contain lstrip()");
    }

    #[test]
    fn test_rstrip_basic() {
        let python_code = r#"
def strip_trailing(s: str) -> str:
    return s.rstrip()
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains("trim_end()"), "Should contain trim_end()");
        assert!(!rust_code.contains("rstrip()"), "Should not contain rstrip()");
    }

    #[test]
    fn test_isalnum_basic() {
        let python_code = r#"
def is_alphanumeric(s: str) -> bool:
    return s.isalnum()
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains("chars()"), "Should contain chars()");
        assert!(
            rust_code.contains("is_alphanumeric()"),
            "Should contain is_alphanumeric()"
        );
        assert!(!rust_code.contains("isalnum()"), "Should not contain isalnum()");
    }

    #[test]
    fn test_string_count_already_working() {
        let python_code = r#"
def count_occurrences(s: str, substring: str) -> int:
    return s.count(substring)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains("matches"), "Should contain matches");
        assert!(rust_code.contains(".count()"), "Should contain .count()");
    }

    #[test]
    fn test_all_phase1_methods_together() {
        let python_code = r#"
def process_string(text: str) -> tuple[str, str, bool, int]:
    leading = text.lstrip()
    trailing = text.rstrip()
    is_alnum = text.isalnum()
    count_a = text.count("a")
    return (leading, trailing, is_alnum, count_a)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains("trim_start()"), "Should contain trim_start()");
        assert!(rust_code.contains("trim_end()"), "Should contain trim_end()");
        assert!(
            rust_code.contains("is_alphanumeric()"),
            "Should contain is_alphanumeric()"
        );
        assert!(rust_code.contains("matches"), "Should contain matches");
    }
}

// ============================================================================
// String Slicing Tests
// ============================================================================

mod string_slicing {
    use super::*;

    #[test]
    fn test_string_last_char() {
        let python_code = r#"
def get_last_char(s: str) -> str:
    return s[-1]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(!rust_code.contains(".to_vec()"), "Should NOT use .to_vec() for strings");
        assert!(
            !rust_code.contains("base.iter()"),
            "Should NOT use .iter() for strings - use .chars() instead"
        );
    }

    #[test]
    fn test_string_last_n_chars() {
        let python_code = r#"
def get_last_n_chars(s: str, n: int) -> str:
    return s[-n:]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains(".chars()"), "Should use .chars() for string slicing");
        assert!(!rust_code.contains(".to_vec()"), "Should NOT use .to_vec() for strings");
        assert!(
            !rust_code.contains("Vec::new()"),
            "Should use String::new() not Vec::new()"
        );
        assert!(rust_code.contains("collect::<String>()"), "Should collect into String");
    }

    #[test]
    fn test_string_all_but_last_n() {
        let python_code = r#"
def get_all_but_last_n(s: str, n: int) -> str:
    return s[:-n]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains(".chars()"), "Should use .chars() for string slicing");
        assert!(rust_code.contains(".take("), "Should use .take() for prefix slicing");
        assert!(!rust_code.contains(".to_vec()"), "Should NOT use .to_vec() for strings");
    }

    #[test]
    fn test_string_reverse() {
        let python_code = r#"
def reverse_string(s: str) -> str:
    return s[::-1]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains(".chars()"),
            "Should use .chars() for string operations"
        );
        assert!(rust_code.contains(".rev()"), "Should use .rev() for reversal");
        assert!(rust_code.contains("collect::<String>()"), "Should collect into String");
    }

    #[test]
    fn test_string_slice_start_stop() {
        let python_code = r#"
def substring(s: str, start: int, stop: int) -> str:
    return s[start:stop]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains(".chars()"), "Should use .chars() for string slicing");
        assert!(
            rust_code.contains(".skip(") || rust_code.contains(".take("),
            "Should use .skip()/.take() for range slicing"
        );
    }

    #[test]
    fn test_string_slice_with_step() {
        let python_code = r#"
def every_nth_char(s: str, n: int) -> str:
    return s[::n]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains(".chars()"), "Should use .chars() for string slicing");
        assert!(
            rust_code.contains(".step_by(") || rust_code.contains("step"),
            "Should handle step parameter"
        );
    }

    #[test]
    fn test_string_slicing_compiles() {
        let python_code = r#"
def test_all_patterns(s: str) -> str:
    last = s[-1]
    last_n = s[-3:]
    without_last = s[:-1]
    reversed = s[::-1]
    middle = s[1:4]
    return reversed
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        std::fs::write("/tmp/test_string_patterns.rs", &rust_code).expect("Failed to write test file");

        let output = std::process::Command::new("rustc")
            .args(["--crate-type", "lib", "/tmp/test_string_patterns.rs"])
            .output()
            .expect("Failed to compile");

        if !output.status.success() {
            panic!(
                "Generated code failed to compile\nSTDERR:\n{}\nGenerated:\n{}",
                String::from_utf8_lossy(&output.stderr),
                rust_code
            );
        }
    }

    #[test]
    fn test_regression_vec_slicing_still_works() {
        let python_code = r#"
def last_elements(arr: list[int]) -> list[int]:
    return arr[-3:]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains("Vec") || rust_code.contains("vec"),
            "Should generate Vec-appropriate code"
        );
    }

    #[test]
    fn test_regression_string_methods_still_work() {
        let python_code = r#"
def process_string(s: str) -> str:
    repeated = s * 3
    return repeated
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains(".repeat("), "String repetition should still work");
    }

    #[test]
    fn test_type_discrimination_string_vs_list() {
        let python_code = r#"
def mixed_types(s: str, arr: list[int]) -> str:
    last_char = s[-1]
    last_elem = arr[-1]
    return last_char
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains("s") || rust_code.contains("string"),
            "Should handle string variable"
        );
        assert!(
            rust_code.contains("arr") || rust_code.contains("list"),
            "Should handle list variable"
        );
    }

    #[test]
    fn test_empty_slice() {
        let python_code = r#"
def full_copy(s: str) -> str:
    return s[:]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains(".to_string()") || rust_code.contains(".clone()"),
            "Full slice should use simple copy"
        );
    }

    #[test]
    fn test_negative_step_reverse() {
        let python_code = r#"
def reverse_every_second(s: str) -> str:
    return s[::-2]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(rust_code.contains(".rev()"), "Negative step should use .rev()");
        assert!(
            rust_code.contains(".step_by(") || rust_code.contains("abs_step"),
            "Should handle step magnitude"
        );
    }
}

// ============================================================================
// F-String Tests
// ============================================================================

mod fstring {
    use super::*;

    #[test]
    fn test_fstring_simple_variable() {
        let python = r#"
def greet(name: str) -> str:
    return f"Hello {name}"
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        let has_format = rust_code.contains("format!") || rust_code.contains("format !");
        assert!(
            has_format,
            "F-string should generate format!() macro.\nGot:\n{}",
            rust_code
        );

        assert!(
            rust_code.contains("\"Hello {}\""),
            "F-string template should be \"Hello {{}}\".\nGot:\n{}",
            rust_code
        );

        assert!(
            rust_code.contains("name"),
            "F-string should pass 'name' to format!().\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_fstring_multiple_variables() {
        let python = r#"
def describe(name: str, age: int) -> str:
    return f"{name} is {age} years old"
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        let has_format = rust_code.contains("format!") || rust_code.contains("format !");
        assert!(has_format, "Should generate format!().\nGot:\n{}", rust_code);

        assert!(
            rust_code.contains("{} is {} years old"),
            "Template should have two {{}}.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_fstring_in_assignment() {
        let python = r#"
def test() -> str:
    name = "Alice"
    message = f"Welcome {name}"
    return message
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        let has_format = rust_code.contains("format!") || rust_code.contains("format !");
        assert!(has_format, "F-string in assignment should work.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_fstring_empty_string() {
        let python = r#"
def test() -> str:
    return f""
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("\"\"") || rust_code.contains("String::new()"),
            "Empty f-string should be empty string.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_fstring_no_interpolation() {
        let python = r#"
def test() -> str:
    return f"Hello World"
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("\"Hello World\""),
            "F-string with no vars should be plain string.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_fstring_with_numbers() {
        let python = r#"
def test(x: int, y: float) -> str:
    return f"x={x}, y={y}"
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        let has_format = rust_code.contains("format!") || rust_code.contains("format !");
        assert!(has_format, "F-string with numbers should work.\nGot:\n{}", rust_code);

        assert!(
            rust_code.contains("x={}, y={}"),
            "Template should preserve literal text.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_fstring_escaped_braces() {
        let python = r#"
def test() -> str:
    return f"{{literal braces}}"
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("{{literal braces}}") || rust_code.contains("{literal braces}"),
            "Escaped braces should be preserved.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_fstring_concatenation() {
        let python = r#"
def test(first: str, last: str) -> str:
    full_name = f"{first}" + f" {last}"
    return full_name
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        let has_format = rust_code.contains("format!") || rust_code.contains("format !");
        assert!(has_format, "Concatenated f-strings should work.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_fstring_in_function_call() {
        let python = r#"
def helper(s: str) -> str:
    return s

def test(name: str) -> str:
    return helper(f"Hello {name}")
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        let has_format = rust_code.contains("format!") || rust_code.contains("format !");
        assert!(
            has_format,
            "F-string as function argument should work.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_fstring_multiline() {
        let python = r#"
def test(name: str, age: int) -> str:
    message = f"""Hello {name}
You are {age} years old"""
    return message
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        let has_format = rust_code.contains("format!") || rust_code.contains("format !");
        assert!(has_format, "Multiline f-string should work.\nGot:\n{}", rust_code);
    }
}
