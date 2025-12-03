//! Consolidated tests for generator transpilation
//!
//! This file consolidates:
//! - generator_test.rs (Basic generators with yield)
//! - generator_expression_test.rs (Generator expressions)
//! - generator_stateful_test.rs (Stateful generators)
//! - generator_gen_coverage_test.rs (Coverage tests)

use depyler_core::DepylerPipeline;

// ============================================================================
// Basic Generators with Yield
// ============================================================================

mod basic_generators {
    use super::*;

    #[test]
    fn test_simple_yield_single_value() {
        let python = r#"
def simple_generator():
    yield 1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn simple_generator") || rust_code.contains("struct SimpleGenerator"),
            "Should have generator function or struct.\nGot:\n{}", rust_code
        );

        let has_iterator = rust_code.contains("Iterator") || rust_code.contains("iter");
        assert!(has_iterator, "Should have Iterator trait.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_yield_multiple_values() {
        let python = r#"
def count_to_three():
    yield 1
    yield 2
    yield 3
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("count_to_three") || rust_code.contains("CountToThree"),
            "Should have generator.\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_generator_with_loop() {
        let python = r#"
def count_up(n: int):
    i = 0
    while i < n:
        yield i
        i = i + 1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("count_up") || rust_code.contains("CountUp"),
            "Should have generator.\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_generator_with_range() {
        let python = r#"
def range_generator(n: int):
    for i in range(n):
        yield i
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("range_generator") || rust_code.contains("RangeGenerator"),
            "Should have generator.\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_generator_with_conditional() {
        let python = r#"
def even_numbers(n: int):
    for i in range(n):
        if i % 2 == 0:
            yield i
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("even_numbers") || rust_code.contains("EvenNumbers"),
            "Should have generator.\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_generator_yielding_expressions() {
        let python = r#"
def squares(n: int):
    for i in range(n):
        yield i * i
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("squares") || rust_code.contains("Squares"),
            "Should have generator.\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_generator_with_local_variables() {
        let python = r#"
def fibonacci(n: int):
    a = 0
    b = 1
    for i in range(n):
        yield a
        temp = a
        a = b
        b = temp + b
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fibonacci") || rust_code.contains("Fibonacci"),
            "Should have generator.\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_generator_yielding_strings() {
        let python = r#"
def string_generator():
    yield "first"
    yield "second"
    yield "third"
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("string_generator") || rust_code.contains("StringGenerator"),
            "Should have generator.\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_generator_with_return() {
        let python = r#"
def limited_generator(n: int):
    for i in range(n):
        if i >= 3:
            return
        yield i
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("limited_generator") || rust_code.contains("LimitedGenerator"),
            "Should have generator.\nGot:\n{}", rust_code
        );
    }
}

// ============================================================================
// Generator Expressions
// ============================================================================

mod generator_expressions {
    use super::*;

    #[test]
    fn test_simple_generator_expression() {
        let python = r#"
def use_gen() -> list:
    gen = (x for x in range(5))
    return list(gen)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains(".into_iter()") || rust_code.contains("0..5"),
            "Should have iterator.\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_generator_expression_with_transform() {
        let python = r#"
def use_gen() -> list:
    gen = (x * 2 for x in range(5))
    return list(gen)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(rust_code.contains(".map("), "Should have .map() transformation.\nGot:\n{}", rust_code);
        assert!(rust_code.contains("* 2") || rust_code.contains("*2"), "Should have multiplication.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_generator_expression_with_filter() {
        let python = r#"
def use_gen() -> list:
    gen = (x for x in range(10) if x % 2 == 0)
    return list(gen)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(rust_code.contains(".filter("), "Should have .filter().\nGot:\n{}", rust_code);
        assert!(rust_code.contains("% 2") && rust_code.contains("== 0"), "Should have modulo check.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_generator_expression_map_and_filter() {
        let python = r#"
def use_gen() -> list:
    gen = (x * 2 for x in range(10) if x > 5)
    return list(gen)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains(".filter(") && rust_code.contains(".map("),
            "Should have both .filter() and .map().\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_generator_expression_in_sum() {
        let python = r#"
def calculate() -> int:
    return sum(x**2 for x in range(5))
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains(".sum()") || rust_code.contains(".sum::<"),
            "Should have .sum() or .sum::<T>().\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_generator_expression_in_max() {
        let python = r#"
def find_max(nums: list) -> int:
    return max(x * 2 for x in nums)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(rust_code.contains(".max()"), "Should have .max().\nGot:\n{}", rust_code);
        assert!(rust_code.contains(".map("), "Should have .map() for transformation.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_generator_expression_with_list_source() {
        let python = r#"
def use_gen(nums: list) -> list:
    gen = (x + 1 for x in nums)
    return list(gen)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains(".iter()") || rust_code.contains(".into_iter()"),
            "Should have iterator over list.\nGot:\n{}", rust_code
        );
        assert!(rust_code.contains("+ 1") || rust_code.contains("+1"), "Should have +1 operation.\nGot:\n{}", rust_code);
    }
}

// ============================================================================
// Stateful Generators
// ============================================================================

mod stateful_generators {
    use super::*;

    #[test]
    fn test_counter_state() {
        let python = r#"
def counter(start: int, end: int):
    current = start
    while current < end:
        yield current
        current = current + 1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Counter") || rust_code.contains("struct counter"),
            "Should have generator state struct.\nGot:\n{}", rust_code
        );
        assert!(rust_code.contains("impl Iterator"), "Should implement Iterator trait.\nGot:\n{}", rust_code);
        assert!(rust_code.contains("current"), "Should track current state.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_multiple_state_variables() {
        let python = r#"
def dual_counter(n: int):
    even = 0
    odd = 1
    for i in range(n):
        if i % 2 == 0:
            yield even
            even = even + 2
        else:
            yield odd
            odd = odd + 2
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(rust_code.contains("struct"), "Should have state struct.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_fibonacci_generator() {
        let python = r#"
def fibonacci(n: int):
    a = 0
    b = 1
    for i in range(n):
        yield a
        temp = a
        a = b
        b = temp + b
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(rust_code.contains("impl Iterator"), "Should implement Iterator.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_accumulator_state() {
        let python = r#"
def running_sum(numbers: list):
    total = 0
    for num in numbers:
        total = total + num
        yield total
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("total") || rust_code.contains("state"),
            "Should maintain accumulator state.\nGot:\n{}", rust_code
        );
    }

    #[test]
    fn test_state_in_nested_loop() {
        let python = r#"
def grid_generator(rows: int, cols: int):
    for i in range(rows):
        for j in range(cols):
            yield (i, j)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(rust_code.contains("impl Iterator"), "Should implement Iterator.\nGot:\n{}", rust_code);
    }
}

// ============================================================================
// Generator Coverage Tests
// ============================================================================

mod generator_coverage {
    use super::*;

    #[test]
    fn test_generator_float_literal_yield() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def float_generator():
    yield 3.14
    yield 2.718
"#;
        let rust_code = pipeline.transpile(python_code).unwrap();
        assert!(rust_code.contains("fn float_generator"));
    }

    #[test]
    fn test_generator_bool_literal_yield() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def bool_generator():
    yield True
    yield False
"#;
        let rust_code = pipeline.transpile(python_code).unwrap();
        assert!(rust_code.contains("fn bool_generator"));
    }

    #[test]
    fn test_generator_bytes_literal_yield() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def bytes_generator():
    yield b"hello"
    yield b"world"
"#;
        let rust_code = pipeline.transpile(python_code).unwrap();
        assert!(rust_code.contains("fn bytes_generator"));
    }

    #[test]
    fn test_generator_none_literal_yield() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def none_generator():
    yield None
"#;
        let rust_code = pipeline.transpile(python_code).unwrap();
        assert!(rust_code.contains("fn none_generator"));
    }

    #[test]
    fn test_generator_in_for_loop() {
        let python = r#"
def simple_range(n: int):
    for i in range(n):
        yield i

def use_generator():
    total = 0
    for value in simple_range(5):
        total = total + value
    return total
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(rust_code.contains("use_generator"), "Should have use_generator function.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_generator_to_list() {
        let python = r#"
def numbers(n: int):
    for i in range(n):
        yield i

def get_list():
    return list(numbers(5))
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(rust_code.contains("get_list"), "Should have get_list function.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_generator_with_complex_logic() {
        let python = r#"
def complex_generator(start: int, end: int):
    current = start
    while current < end:
        if current % 2 == 0:
            yield current * 2
        else:
            yield current
        current = current + 1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("complex_generator") || rust_code.contains("ComplexGenerator"),
            "Should have generator.\nGot:\n{}", rust_code
        );
    }
}
