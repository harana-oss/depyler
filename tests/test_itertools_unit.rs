
// Module: itertools - Python itertools module validation
// pending

use crate::test_helpers::transpile_and_check;

// 
#[test]
fn test_count() {
    let python = r#"
import itertools

def get_counter(start: int, step: int) -> itertools.count:
    return itertools.count(start, step)
"#;

    let result = transpile_and_check(python, &[]);

    // Should create infinite counter
    assert!(result.contains("iter") || result.contains("range") || result.contains("successors"));
}

#[test]
fn test_cycle() {
    let python = r#"
import itertools

def cycle_items(items: list) -> itertools.cycle:
    return itertools.cycle(items)
"#;

    let result = transpile_and_check(python, &[]);

    // Should create cycling iterator
    assert!(result.contains("cycle") || result.contains("iter") || result.contains("repeat"));
}

#[test]
fn test_repeat() {
    let python = r#"
import itertools

def repeat_value(value: int, times: int) -> itertools.repeat:
    return itertools.repeat(value, times)
"#;

    let result = transpile_and_check(python, &[]);

    // Should create repeating iterator
    assert!(result.contains("repeat") && result.contains("take"));
}

// 
#[test]
fn test_chain() {
    let python = r#"
import itertools

def chain_iterables(a: list, b: list) -> itertools.chain:
    return itertools.chain(a, b)
"#;

    let result = transpile_and_check(python, &[]);

    // Should chain multiple iterables
    assert!(result.contains("chain") && result.contains("into_iter"));
}

#[test]
fn test_islice() {
    let python = r#"
import itertools

def slice_iter(items: list, start: int, stop: int) -> itertools.islice:
    return itertools.islice(items, start, stop)
"#;

    let result = transpile_and_check(python, &[]);

    // Should slice iterator
    assert!(result.contains("skip") && result.contains("take"));
}

#[test]
fn test_takewhile() {
    let python = r#"
import itertools

def take_while_positive(items: list) -> itertools.takewhile:
    return itertools.takewhile(lambda x: x > 0, items)
"#;

    let result = transpile_and_check(python, &[]);

    // Should take while condition is true
    assert!(result.contains("take_while"));
}

// Total: 6 comprehensive tests for itertools module
// Coverage: count, cycle, repeat, chain, islice, takewhile
// Iterator combinatorics and lazy evaluation
