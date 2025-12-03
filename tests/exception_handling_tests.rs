//! Consolidated tests for try/except/finally error handling
//!
//! This file consolidates:
//! - try_except_test.rs (Phase 1: Simple try/except)
//! - try_except_multiple_test.rs (Phase 2: Multiple exception handlers)
//! - try_except_finally_test.rs (Phase 3: Finally clause)

use depyler_core::DepylerPipeline;

// ============================================================================
// Phase 1: Simple Try/Except Blocks
// ============================================================================

mod simple_try_except {
    use super::*;

    #[test]
    fn test_simple_try_except() {
        let python = r#"
def safe_divide(a: int, b: int) -> int:
    try:
        return a // b
    except:
        return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn safe_divide"),
            "Should have safe_divide function.\nGot:\n{}",
            rust_code
        );

        let has_error_handling =
            rust_code.contains("Result") || rust_code.contains("match") || rust_code.contains("if let");
        assert!(
            has_error_handling,
            "Should have error handling pattern.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_try_except_with_binding() {
        let python = r#"
def parse_number(s: str) -> int:
    try:
        return int(s)
    except ValueError as e:
        return -1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn parse_number"),
            "Should have parse_number function.\nGot:\n{}",
            rust_code
        );

        let has_error_handling =
            rust_code.contains("Result") || rust_code.contains("match") || rust_code.contains("Err");
        assert!(has_error_handling, "Should have error handling.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_return_in_try() {
        let python = r#"
def get_value(data: dict, key: str) -> str:
    try:
        return data[key]
    except KeyError:
        return "default"
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn get_value"),
            "Should have get_value function.\nGot:\n{}",
            rust_code
        );

        let has_return = rust_code.contains("return") || rust_code.contains("->");
        assert!(has_return, "Should have return handling.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_return_in_except() {
        let python = r#"
def safe_operation(x: int) -> int:
    try:
        result = x * 2
        return result
    except:
        return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn safe_operation"),
            "Should have safe_operation function.\nGot:\n{}",
            rust_code
        );

        let has_error_handling =
            rust_code.contains("Result") || rust_code.contains("match") || rust_code.contains("if let");
        assert!(has_error_handling, "Should have error handling.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_nested_try_except() {
        let python = r#"
def nested_operation(x: int, y: int) -> int:
    try:
        try:
            return x // y
        except ValueError:
            return x
    except:
        return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn nested_operation"),
            "Should have nested_operation function.\nGot:\n{}",
            rust_code
        );

        let has_error_handling = rust_code.contains("Result") || rust_code.contains("match");
        assert!(has_error_handling, "Should have error handling.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_in_function() {
        let python = r#"
def process_data(data: list) -> int:
    count = 0
    try:
        count = len(data)
    except:
        count = 0
    return count
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn process_data"),
            "Should have process_data function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("count"),
            "Should have count variable.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_try_except_specific_exception() {
        let python = r#"
def convert_to_int(s: str) -> int:
    try:
        return int(s)
    except ValueError:
        return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn convert_to_int"),
            "Should have convert_to_int function.\nGot:\n{}",
            rust_code
        );

        let has_error_handling =
            rust_code.contains("Result") || rust_code.contains("match") || rust_code.contains("Err");
        assert!(has_error_handling, "Should have error handling.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_with_pass() {
        let python = r#"
def ignore_errors(x: int) -> int:
    try:
        result = x * 2
        return result
    except:
        pass
    return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn ignore_errors"),
            "Should have ignore_errors function.\nGot:\n{}",
            rust_code
        );

        let has_error_handling =
            rust_code.contains("Result") || rust_code.contains("match") || rust_code.contains("if let");
        assert!(has_error_handling, "Should have error handling.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_multiple_statements_in_try() {
        let python = r#"
def calculate(x: int, y: int) -> int:
    try:
        a = x * 2
        b = y * 3
        return a + b
    except:
        return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn calculate"),
            "Should have calculate function.\nGot:\n{}",
            rust_code
        );

        let has_vars = rust_code.contains("a") && rust_code.contains("b");
        assert!(has_vars, "Should have multiple variables.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_multiple_statements_in_except() {
        let python = r#"
def safe_process(x: int) -> int:
    try:
        return x // 2
    except:
        default = 0
        return default
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn safe_process"),
            "Should have safe_process function.\nGot:\n{}",
            rust_code
        );

        let has_error_handling =
            rust_code.contains("Result") || rust_code.contains("match") || rust_code.contains("if let");
        assert!(has_error_handling, "Should have error handling.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_accessing_exception_message() {
        let python = r#"
def get_error_message(s: str) -> str:
    try:
        return int(s)
    except Exception as e:
        return str(e)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn get_error_message"),
            "Should have get_error_message function.\nGot:\n{}",
            rust_code
        );

        let has_error_binding = rust_code.contains("Err") || rust_code.contains("match");
        assert!(has_error_binding, "Should have error binding.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_bare_except() {
        let python = r#"
def catch_all(x: int) -> int:
    try:
        return x * 2
    except:
        return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn catch_all"),
            "Should have catch_all function.\nGot:\n{}",
            rust_code
        );

        let has_error_handling =
            rust_code.contains("Result") || rust_code.contains("match") || rust_code.contains("if let");
        assert!(has_error_handling, "Should have error handling.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_with_computation() {
        let python = r#"
def compute_ratio(a: int, b: int) -> float:
    try:
        ratio = a / b
        return ratio
    except ZeroDivisionError:
        return 0.0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn compute_ratio"),
            "Should have compute_ratio function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("ratio"),
            "Should have ratio variable.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_try_except_with_side_effects() {
        let python = r#"
def log_operation(x: int) -> int:
    try:
        result = x * 2
        print(result)
        return result
    except:
        print("error")
        return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn log_operation"),
            "Should have log_operation function.\nGot:\n{}",
            rust_code
        );

        let has_print = rust_code.contains("print") || rust_code.contains("println");
        assert!(has_print, "Should have print statements.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_with_variable_assignment() {
        let python = r#"
def assign_safely(x: int) -> int:
    result = 0
    try:
        result = x * 2
    except:
        result = -1
    return result
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn assign_safely"),
            "Should have assign_safely function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("result"),
            "Should have result variable.\nGot:\n{}",
            rust_code
        );
    }
}

// ============================================================================
// Phase 2: Multiple Exception Handlers
// ============================================================================

mod multiple_exception_handlers {
    use super::*;

    #[test]
    fn test_two_exception_types() {
        let python = r#"
def parse_data(data: str) -> int:
    try:
        return int(data)
    except ValueError:
        return -1
    except TypeError:
        return -2
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn parse_data"),
            "Should have parse_data function.\nGot:\n{}",
            rust_code
        );

        let has_error_handling = rust_code.contains("match") || rust_code.contains("if let");
        assert!(has_error_handling, "Should have error handling.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_three_exception_types() {
        let python = r#"
def safe_operation(x: int) -> int:
    try:
        result = x * 2
        return result
    except ValueError:
        return -1
    except KeyError:
        return -2
    except IndexError:
        return -3
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn safe_operation"),
            "Should have safe_operation function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_specific_then_bare_except() {
        let python = r#"
def handle_errors(data: str) -> str:
    try:
        return data.upper()
    except ValueError:
        return "value_error"
    except:
        return "unknown_error"
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn handle_errors"),
            "Should have handle_errors function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_multiple_exceptions_one_handler() {
        let python = r#"
def process(data: str) -> int:
    try:
        return int(data)
    except (ValueError, TypeError):
        return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn process"),
            "Should have process function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_exceptions_with_different_actions() {
        let python = r#"
def calculate(a: int, b: int) -> int:
    try:
        return a // b
    except ZeroDivisionError:
        print("Division by zero")
        return 0
    except TypeError:
        print("Type error")
        return -1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn calculate"),
            "Should have calculate function.\nGot:\n{}",
            rust_code
        );

        let has_print = rust_code.contains("print") || rust_code.contains("println");
        assert!(has_print, "Should have print statements.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_nested_with_multiple_handlers() {
        let python = r#"
def nested_operation(x: int, y: int) -> int:
    try:
        try:
            return x // y
        except ValueError:
            return x
    except ZeroDivisionError:
        return 0
    except:
        return -1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn nested_operation"),
            "Should have nested_operation function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_handlers_with_return_values() {
        let python = r#"
def get_value(data: dict, key: str) -> str:
    try:
        return data[key]
    except KeyError:
        return "default_key"
    except TypeError:
        return "default_type"
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn get_value"),
            "Should have get_value function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_exception_order_matters() {
        let python = r#"
def specific_first(x: int) -> str:
    try:
        result = x * 2
        return str(result)
    except ValueError:
        return "value_error"
    except Exception:
        return "general_error"
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn specific_first"),
            "Should have specific_first function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_different_exception_variables() {
        let python = r#"
def handle_with_vars(data: str) -> str:
    try:
        return data.upper()
    except ValueError as ve:
        return str(ve)
    except TypeError as te:
        return str(te)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn handle_with_vars"),
            "Should have handle_with_vars function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_handlers_with_side_effects() {
        let python = r#"
def log_errors(x: int) -> int:
    try:
        result = x * 2
        return result
    except ValueError:
        print("ValueError occurred")
        return 0
    except TypeError:
        print("TypeError occurred")
        return -1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn log_errors"),
            "Should have log_errors function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_multiple_handlers_with_pass() {
        let python = r#"
def ignore_specific(x: int) -> int:
    try:
        return x * 2
    except ValueError:
        pass
    except TypeError:
        pass
    return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn ignore_specific"),
            "Should have ignore_specific function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_handlers_accessing_exception_message() {
        let python = r#"
def get_error_message(data: str) -> str:
    try:
        return int(data)
    except ValueError as e:
        return "ValueError: " + str(e)
    except TypeError as e:
        return "TypeError: " + str(e)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn get_error_message"),
            "Should have get_error_message function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_handlers_with_computations() {
        let python = r#"
def compute_fallback(x: int, y: int) -> int:
    try:
        return x // y
    except ZeroDivisionError:
        return x * 2
    except TypeError:
        return x + y
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn compute_fallback"),
            "Should have compute_fallback function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_reraise_in_handler() {
        let python = r#"
def reraise_specific(x: int) -> int:
    try:
        return x * 2
    except ValueError:
        print("ValueError caught")
        raise
    except TypeError:
        return 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn reraise_specific"),
            "Should have reraise_specific function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_handlers_with_variable_assignment() {
        let python = r#"
def assign_in_handlers(x: int) -> int:
    result = 0
    try:
        result = x * 2
    except ValueError:
        result = -1
    except TypeError:
        result = -2
    return result
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn assign_in_handlers"),
            "Should have assign_in_handlers function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_handlers_calling_functions() {
        let python = r#"
def handle_with_calls(x: int) -> int:
    try:
        return x * 2
    except ValueError:
        return abs(x)
    except TypeError:
        return len(str(x))
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn handle_with_calls"),
            "Should have handle_with_calls function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_handlers_with_conditionals() {
        let python = r#"
def conditional_handlers(x: int, flag: bool) -> int:
    try:
        return x * 2
    except ValueError:
        if flag:
            return 0
        else:
            return -1
    except TypeError:
        return -2
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn conditional_handlers"),
            "Should have conditional_handlers function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_handlers_modifying_state() {
        let python = r#"
def modify_state(x: int) -> int:
    count = 0
    try:
        count = x * 2
        return count
    except ValueError:
        count = count + 1
        return count
    except TypeError:
        count = count + 2
        return count
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn modify_state"),
            "Should have modify_state function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_chained_exception_handlers() {
        let python = r#"
def chained_handling(data: list) -> int:
    try:
        return data[0]
    except IndexError:
        try:
            return len(data)
        except TypeError:
            return -1
    except ValueError:
        return -2
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn chained_handling"),
            "Should have chained_handling function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_complex_exception_pattern() {
        let python = r#"
def complex_handling(a: int, b: int, c: int) -> int:
    result = 0
    try:
        temp = a // b
        result = temp * c
        return result
    except ZeroDivisionError:
        result = a * c
        return result
    except ValueError:
        result = b * c
        return result
    except TypeError:
        result = -1
        return result
    except:
        result = -2
        return result
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn complex_handling"),
            "Should have complex_handling function.\nGot:\n{}",
            rust_code
        );
    }
}

// ============================================================================
// Phase 3: Finally Clause
// ============================================================================

mod finally_clause {
    use super::*;

    #[test]
    fn test_simple_try_finally() {
        let python = r#"
def cleanup_operation(x: int) -> int:
    try:
        result = x * 2
        return result
    finally:
        print("cleanup")
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn cleanup_operation"),
            "Should have cleanup_operation function.\nGot:\n{}",
            rust_code
        );

        let has_print = rust_code.contains("print") || rust_code.contains("println");
        assert!(has_print, "Should have cleanup print.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_try_except_finally() {
        let python = r#"
def safe_divide(a: int, b: int) -> int:
    try:
        return a // b
    except ZeroDivisionError:
        return 0
    finally:
        print("operation complete")
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn safe_divide"),
            "Should have safe_divide function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_finally_with_return_in_try() {
        let python = r#"
def process_with_cleanup(x: int) -> int:
    count = 0
    try:
        count = x * 2
        return count
    finally:
        count = count + 1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn process_with_cleanup"),
            "Should have process_with_cleanup function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_finally_with_exception() {
        let python = r#"
def handle_with_cleanup(data: str) -> int:
    result = 0
    try:
        result = int(data)
        return result
    except ValueError:
        result = -1
        return result
    finally:
        print("cleanup executed")
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn handle_with_cleanup"),
            "Should have handle_with_cleanup function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_finally_with_side_effects() {
        let python = r#"
def log_and_cleanup(x: int) -> int:
    try:
        result = x * 2
        print("processing")
        return result
    finally:
        print("cleanup started")
        print("cleanup finished")
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn log_and_cleanup"),
            "Should have log_and_cleanup function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_finally_with_variable_assignment() {
        let python = r#"
def track_execution(x: int) -> int:
    executed = False
    try:
        result = x * 2
        return result
    finally:
        executed = True
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn track_execution"),
            "Should have track_execution function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_try_except_else_finally() {
        let python = r#"
def complete_pattern(x: int) -> int:
    try:
        result = x * 2
    except ValueError:
        result = -1
    else:
        result = result + 1
    finally:
        print("done")
    return result
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn complete_pattern"),
            "Should have complete_pattern function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_nested_try_finally() {
        let python = r#"
def nested_cleanup(x: int) -> int:
    try:
        try:
            result = x * 2
            return result
        finally:
            print("inner cleanup")
    finally:
        print("outer cleanup")
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn nested_cleanup"),
            "Should have nested_cleanup function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_finally_with_resource_cleanup() {
        let python = r#"
def open_and_process(filename: str) -> str:
    file = None
    try:
        file = open(filename)
        return file.read()
    except IOError:
        return "error"
    finally:
        if file:
            file.close()
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn open_and_process"),
            "Should have open_and_process function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_finally_with_multiple_statements() {
        let python = r#"
def complex_cleanup(x: int, y: int) -> int:
    a = 0
    b = 0
    try:
        a = x * 2
        b = y * 3
        return a + b
    except ValueError:
        return -1
    finally:
        print("cleanup a")
        print("cleanup b")
        a = 0
        b = 0
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn complex_cleanup"),
            "Should have complex_cleanup function.\nGot:\n{}",
            rust_code
        );
    }
}

// ============================================================================
// Multiple Exception Types Tests
// ============================================================================

mod multiple_exception_types {
    use super::*;

    #[test]
    fn test_multiple_exception_handlers_compiles() {
        let python = r#"
def parse_value(s: str) -> int:
    try:
        return int(s)
    except ValueError:
        print("Value error")
        return -1
    except TypeError:
        print("Type error")
        return -2
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation should succeed: {:?}", result.err());

        let rust_code = result.unwrap();
        assert!(rust_code.contains("Value error"), "Should contain ValueError handler");
        assert!(rust_code.contains("Type error"), "Should contain TypeError handler");

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_depyler_0361.rs");
        std::fs::write(&test_file, &rust_code).expect("Should write file");

        let output = std::process::Command::new("rustc")
            .arg("--crate-type=lib")
            .arg("--edition=2021")
            .arg(&test_file)
            .arg("-o")
            .arg(temp_dir.join("test_depyler_0361.rlib"))
            .output()
            .expect("Should run rustc");

        assert!(
            output.status.success(),
            "Generated code should compile!\nGenerated:\n{}\nErrors:\n{}",
            rust_code,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_tuple_exception_types_compiles() {
        let python = r#"
def multi_except(s: str) -> int:
    try:
        return int(s)
    except (ValueError, TypeError):
        print("Error occurred")
        return -1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation should succeed: {:?}", result.err());

        let rust_code = result.unwrap();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_depyler_0362.rs");
        std::fs::write(&test_file, &rust_code).expect("Should write file");

        let output = std::process::Command::new("rustc")
            .arg("--crate-type=lib")
            .arg("--edition=2021")
            .arg(&test_file)
            .arg("-o")
            .arg(temp_dir.join("test_depyler_0362.rlib"))
            .output()
            .expect("Should run rustc");

        assert!(
            output.status.success(),
            "Generated code should compile!\nGenerated:\n{}\nErrors:\n{}",
            rust_code,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
