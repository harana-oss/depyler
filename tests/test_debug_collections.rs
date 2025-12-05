//! Debug specific collection features

use crate::test_helpers;
use crate::test_helpers::transpile_and_check;

#[test]
#[ignore]
fn test_list_comprehension() {
    let python_code = r#"
def test_comprehension():
    squares = [x * x for x in range(5)]
    return squares
"#;
    transpile_and_check(python_code, &["fn test_comprehension"]);
}

#[test]
fn test_extend_method() {
    let python_code = r#"
def test_extend():
    numbers = [1, 2, 3]
    numbers.extend([7, 8])
    return numbers
"#;
    transpile_and_check(python_code, &["fn test_extend", "extend"]);
}

#[test]
fn test_negative_indexing() {
    let python_code = r#"
def test_negative():
    numbers = [1, 2, 3, 4, 5]
    last = numbers[-1]
    return last
"#;
    transpile_and_check(python_code, &["fn test_negative"]);
}