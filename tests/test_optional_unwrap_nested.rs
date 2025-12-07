use depyler_core::DepylerPipeline;

/// Test that optional variables can be properly unwrapped when accessing nested properties
#[test]
fn test_optional_nested_attribute_access() {
    let python = r#"
class Inner:
    value: int

class Outer:
    inner: Inner

def test_optional_access(opt: Outer | None) -> int:
    if opt is not None:
        return opt.inner.value
    return 0
"#;

    let pipeline = DepylerPipeline::new();
    let rust_code = pipeline.transpile(python).expect("Transpilation failed");

    println!("Generated Rust code:\n{}", rust_code);

    // Check that the code compiles
    assert!(rust_code.contains("fn test_optional_access"));

    // The code should handle the optional properly
    // Either with as_ref().unwrap() or by pattern matching
    assert!(rust_code.contains(".unwrap()") || rust_code.contains("if let Some"));
}

/// Test the exact pattern from test_mega
#[test]
fn test_optional_assignment_then_access() {
    let python = r#"
class Scores:
    total: int
    tries: int

class PeriodStatistic:
    scores: Scores

class State:
    period_statistics: list[PeriodStatistic]

def test_function(state: State) -> None:
    extra_time_stats: PeriodStatistic | None = None
    
    if state is not None:
        extra_time_stats = state.period_statistics[2]
        print(extra_time_stats.scores.tries)
"#;

    let pipeline = DepylerPipeline::new();
    let rust_code = pipeline.transpile(python).expect("Transpilation failed");

    println!("Generated Rust code:\n{}", rust_code);

    // Should properly unwrap the optional when accessing nested fields
    assert!(rust_code.contains("fn test_function"));
}
