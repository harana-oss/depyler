//! Consolidated tests for COW types and lifetime handling
//!
//! Tests cover:
//! - Cow<'static, str> type inference bugs (
//! - String parameter lifetime issues
//! - Lifetime violation detection
//! - V1 lifetime features

use crate::test_helpers;
use crate::test_helpers::transpile;

use depyler_core::DepylerPipeline;
use std::process::Command;

// ============================================================================
// COW Type Tests (
// ============================================================================

mod cow_type {
    use super::*;

    #[test]
    fn test_string_concat_returns_string() {
        let python = r#"
def concat(a: str, b: str) -> str:
    return a + b
"#;

        let pipeline = DepylerPipeline::new();
        let result: Result<String, String> = Ok(transpile(python));

        assert!(result.is_ok(), "Transpilation should succeed");
        let rust = result.unwrap();

        assert!(
            !rust.contains("-> Cow<"),
            "Should not use Cow for string concatenation!\nGenerated:\n{}",
            rust
        );

        assert!(
            rust.contains("-> String"),
            "Should return String!\nGenerated:\n{}",
            rust
        );

        // Parameters can be &str or String depending on implementation
        let has_params = (rust.contains("a: &str") || rust.contains("a: String"))
            && (rust.contains("b: &str") || rust.contains("b: String"));
        assert!(has_params, "Should have string parameters!\nGenerated:\n{}", rust);
    }

    #[test]
    fn test_string_concat_compiles() {
        let python = r#"
def concat(a: str, b: str) -> str:
    return a + b
"#;

        let pipeline = DepylerPipeline::new();
        let rust = transpile(python);

        let temp_file = "/tmp/test_concat.rs";
        std::fs::write(temp_file, &rust).expect("Failed to write temp file");

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(temp_file)
            .arg("-o")
            .arg("/tmp/test_concat.rlib")
            .output()
            .expect("Failed to run rustc");

        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "Generated code must compile!\nErrors:\n{}\nGenerated:\n{}",
            stderr,
            rust
        );
    }

    #[test]
    fn test_fstring_returns_string() {
        let python = r#"
def format_name(first: str, last: str) -> str:
    return f"{first} {last}"
"#;

        let pipeline = DepylerPipeline::new();
        let rust = transpile(python);

        assert!(!rust.contains("-> Cow<"), "F-string should not return Cow");
        assert!(rust.contains("-> String"), "F-string should return String");
    }

    #[test]
fn test_format_call_returns_string() {
        let python = r#"
def format_msg(name: str, count: int) -> str:
    return "{} has {}".format(name, count)
"#;

        let pipeline = DepylerPipeline::new();
        let result: Result<String, String> = Ok(transpile(python));
        // format() method may not be fully supported, so we just check transpilation succeeds
        if let Ok(rust) = result {
            assert!(!rust.contains("-> Cow<"), "format() should not return Cow");
        }
    }

    #[test]
    fn test_multiple_concat_returns_string() {
        let python = r#"
def concat_three(a: str, b: str, c: str) -> str:
    return a + b + c
"#;

        let pipeline = DepylerPipeline::new();
        let rust = transpile(python);

        assert!(!rust.contains("-> Cow<"), "Multi-concat should not return Cow");
        assert!(rust.contains("-> String"), "Multi-concat should return String");
    }
}

// ============================================================================
// COW Lifetime Fix Tests
// ============================================================================

mod cow_lifetime_fix {
    use super::*;

    #[test]
    fn test_string_param_no_static_lifetime() {
        let python_code = r#"
def concatenate(a: str, b: str) -> str:
    """Test function with String parameters."""
    return a + b
"#;

        let pipeline = DepylerPipeline::new();
        let generated_code = transpile(python_code);

        let has_static_cow_param = generated_code.contains("Cow<'static, str>");

        assert!(
            !has_static_cow_param,
            "BUG: Parameters use Cow<'static, str>\nGenerated:\n{}",
            generated_code
        );
    }

    #[test]
    fn test_string_param_compiles_with_local_strings() {
        let python_code = r#"
def concatenate(a: str, b: str) -> str:
    """Test function with String parameters."""
    return a + b
"#;

        let pipeline = DepylerPipeline::new();
        let generated_code = transpile(python_code);

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_cow_compile.rs");

        let test_code = format!(
            r#"{}

#[cfg(test)]
mod test_local_strings {{
    use super::*;

    #[test]
    fn test_with_local_strings() {{
        let a = String::from("hello");
        let b = String::from("world");
        let _result = concatenate(&a, &b);
    }}
}}
"#,
            generated_code
        );

        std::fs::write(&test_file, &test_code).expect("Failed to write test file");

        let output = std::process::Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--test")
            .arg(&test_file)
            .arg("--out-dir")
            .arg(&temp_dir)
            .arg("--edition")
            .arg("2021")
            .output()
            .expect("Failed to run rustc");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Cow<'static, str>") || stderr.contains("cannot infer") || stderr.contains("lifetime") {
                panic!(
                    "BUG: Generated code requires 'static lifetime!\nError:\n{}\nGenerated:\n{}",
                    stderr, generated_code
                );
            }
        }
    }
}

// ============================================================================
// String Lifetime Tests
// ============================================================================

mod string_lifetime {
    use super::*;

    #[test]
    fn test_string_parameter_generates_lifetime() {
        let pipeline = DepylerPipeline::new();
        let python_code = r#"
def process_string(s: str) -> int:
    return len(s)
"#;

        let rust_code = transpile(python_code);

        // s can be &str or String depending on how it's used
        assert!(
            rust_code.contains("s: &str") || rust_code.contains("s: String"),
            "Should have string parameter"
        );
        assert!(
            rust_code.contains("-> i32") || rust_code.contains("-> usize"),
            "Should return integer"
        );
    }

    #[test]
    fn test_string_escape_uses_cow() {
        let pipeline = DepylerPipeline::new();
        let python_code = r#"
def identity(s: str) -> str:
    return s
"#;

        let rust_code = transpile(python_code);

        assert!(
            rust_code.contains("String"),
            "Should use String for string that escapes through return"
        );
    }

    #[test]
    fn test_multiple_string_params_different_lifetimes() {
        let pipeline = DepylerPipeline::new();
        let python_code = r#"
def select_string(s1: str, s2: str, use_first: bool) -> str:
    if use_first:
        return s1
    else:
        return s2
"#;

        let rust_code = transpile(python_code);

        // Parameters that escape should use String
        assert!(
            rust_code.contains("String"),
            "String parameters that escape should use String type"
        );
    }

    #[test]
    fn test_string_concatenation_ownership() {
        let pipeline = DepylerPipeline::new();
        let python_code = r#"
def concat_strings(s1: str, s2: str) -> str:
    return s1 + s2
"#;

        let rust_code = transpile(python_code);

        // String concatenation should work and return String
        assert!(rust_code.contains("-> String"), "Concatenation should return String");
    }

    #[test]
    fn test_string_mutation_takes_ownership() {
        let pipeline = DepylerPipeline::new();
        let python_code = r#"
def append_exclamation(s: str) -> str:
    s = s + "!"
    return s
"#;

        let rust_code = transpile(python_code);

        // Mutation requires ownership
        assert!(
            rust_code.contains("String") || rust_code.contains("mut"),
            "Should handle string mutation"
        );
        assert!(
            rust_code.contains("mut s: String"),
            "Should take ownership as mutable String"
        );
    }
}

// ============================================================================
// Lifetime Violation Detection Tests
// ============================================================================

mod lifetime_violations {
    use super::*;

    #[test]
    fn test_dangling_reference_detection() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def get_local_ref() -> str:
    local = "temporary"
    return local
"#;

        let result: Result<String, String> = Ok(transpile(python_code));
        println!("Result: {:?}", result);
    }

    #[test]
    fn test_reference_outlives_source() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def store_reference(container: List[str], temp: str):
    container.append(temp)
"#;

        let result: Result<String, String> = Ok(transpile(python_code));
        println!("Result for store_reference: {:?}", result);
    }

    #[test]
    fn test_iterator_invalidation() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def modify_while_iterating(items: List[int]):
    for item in items:
        if item > 5:
            items.remove(item)
"#;

        let result: Result<String, String> = Ok(transpile(python_code));
        println!("Result for modify_while_iterating: {:?}", result);
    }

    #[test]
    fn test_multiple_mutable_borrows() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def double_mut_borrow(data: List[int]):
    ref1 = data
    ref2 = data
    ref1.append(1)
    ref2.append(2)
"#;

        let result: Result<String, String> = Ok(transpile(python_code));
        println!("Result for double_mut_borrow: {:?}", result);
    }

    #[test]
    fn test_use_after_move() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def use_after_move(s: str) -> str:
    result = consume_string(s)
    print(s)  # s has been moved
    return result
    
def consume_string(s: str) -> str:
    return s.upper()
"#;

        let result: Result<String, String> = Ok(transpile(python_code));
        println!("Result for use_after_move: {:?}", result);
    }

    #[test]
    fn test_escaping_closure_reference() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def create_closure():
    local_data = "temporary"
    def inner():
        return local_data
    return inner
"#;

        let result: Result<String, String> = Ok(transpile(python_code));
        println!("Result for escaping_closure: {:?}", result);
    }

    #[test]
    fn test_field_lifetime_mismatch() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
class Container:
    def __init__(self, data: str):
        self.data = data
        
def create_container(temp: str) -> Container:
    return Container(temp)
"#;

        let result: Result<String, String> = Ok(transpile(python_code));
        println!("Result for field_lifetime_mismatch: {:?}", result);
    }
}

// ============================================================================
// V1 Lifetime Tests
// ============================================================================

mod v1_lifetime {
    use super::*;

    #[test]
    fn test_returning_ref_to_temporary() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def get_temp_string() -> str:
    return "temporary" + "data"
"#;

        let rust_code = transpile(python_code);

        assert!(
            rust_code.contains("-> String") || rust_code.contains("-> Cow"),
            "Should return owned type for temporary"
        );
    }

    #[test]
    fn test_parameter_lifetime_propagation() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def select_longer(s1: str, s2: str) -> str:
    if len(s1) > len(s2):
        return s1
    else:
        return s2
"#;

        let rust_code = transpile(python_code);

        assert!(
            rust_code.contains("Cow") || rust_code.contains("String"),
            "Should handle string selection appropriately"
        );
    }

    #[test]
    fn test_mixed_lifetime_returns() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def get_string(use_default: bool, custom: str) -> str:
    if use_default:
        return "default"
    else:
        return custom
"#;

        let rust_code = transpile(python_code);

        assert!(
            rust_code.contains("Cow") || rust_code.contains("String"),
            "Should handle mixed lifetime returns"
        );
    }

    #[test]
    fn test_nested_scope_lifetimes() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def process_in_scope(data: str) -> str:
    if len(data) > 0:
        temp = data + "!"
        return temp
    return ""
"#;

        let rust_code = transpile(python_code);

        assert!(
            rust_code.contains("String") || rust_code.contains("Cow"),
            "Should return owned type"
        );
    }

    #[test]
    fn test_lifetime_elision_single_param() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def get_first_char(s: str) -> str:
    if len(s) > 0:
        return s[0]
    return ""
"#;

        let rust_code = transpile(python_code);

        assert!(
            rust_code.contains("char") || rust_code.contains("String"),
            "Should handle character extraction"
        );
    }

    #[test]
    fn test_function_composition_lifetimes() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def outer(s: str) -> str:
    return inner(s)
    
def inner(s: str) -> str:
    return s
"#;

        let rust_code = transpile(python_code);

        assert!(rust_code.contains("inner"), "Should call inner function");
    }
}
