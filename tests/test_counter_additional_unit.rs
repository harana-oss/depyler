
// Module: collections.Counter - Additional Counter methods
// pending

use crate::test_helpers::transpile_and_check;

// 
#[test]
fn test_most_common() {
    let python = r#"
from collections import Counter

def get_top_items(counter: Counter, n: int) -> list:
    return counter.most_common(n)
"#;

    let result = transpile_and_check(python, &[]);

    // Should return n most common elements
    assert!(result.contains("sort") || result.contains("most_common"));
}

#[test]
fn test_elements() {
    let python = r#"
from collections import Counter

def get_all_elements(counter: Counter) -> list:
    return list(counter.elements())
"#;

    let result = transpile_and_check(python, &[]);

    // Should return iterator over elements (repeated by count)
    assert!(result.contains("elements") || result.contains("flat_map") || result.contains("repeat"));
}

#[test]
fn test_total() {
    let python = r#"
from collections import Counter

def get_total(counter: Counter) -> int:
    return counter.total()
"#;

    let result = transpile_and_check(python, &[]);

    // Should return sum of all counts
    assert!(result.contains("sum") || result.contains("values"));
}

// Total: 3 tests for Counter additional methods
// Coverage: most_common(), elements(), total()
