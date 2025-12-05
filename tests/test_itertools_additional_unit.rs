// Module: itertools - Additional iterator functions

use crate::test_helpers::transpile_and_check;

// Note: dropwhile, accumulate, compress were already implemented
// This commit adds 3 NEW functions: zip_longest, filterfalse, starmap

//
#[test]
#[ignore]
fn test_zip_longest() {
    let python = r#"
import itertools

def zip_with_fill(a: list, b: list) -> list:
    return list(itertools.zip_longest(a, b, fillvalue=None))
"#;

    let result = transpile_and_check(python, &[]);

    // Should zip with fill values for shorter iterator
    assert!(result.contains("zip") && (result.contains("fillvalue") || result.contains("None")));
}

//
#[test]
#[ignore]
fn test_filterfalse() {
    let python = r#"
import itertools

def filter_not(items: list, pred) -> list:
    return list(itertools.filterfalse(pred, items))
"#;

    let result = transpile_and_check(python, &[]);

    // Should filter elements where predicate is false
    assert!(result.contains("filter") && result.contains("!"));
}

//
#[test]
#[ignore]
fn test_starmap() {
    let python = r#"
import itertools

def apply_pairs(func, pairs: list) -> list:
    return list(itertools.starmap(func, pairs))
"#;

    let result = transpile_and_check(python, &[]);

    // Should unpack arguments from tuples
    assert!(result.contains("map") && (result.contains("*") || result.contains("unpack")));
}

// Total: 3 NEW itertools functions (dropwhile, accumulate, compress already existed)
// Coverage: zip_longest(), filterfalse(), starmap()
