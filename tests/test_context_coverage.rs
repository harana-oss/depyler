//! Targeted coverage tests for context.rs module

use crate::test_helpers;
use crate::test_helpers::transpile_and_check;

#[test]
fn test_nested_scope_management() {
    let python_code = r#"
def outer():
    x = 1
    if True:
        y = 2
        z = x + y
    return x
"#;
    transpile_and_check(python_code, &["fn outer"]);
}

#[test]
fn test_variable_shadowing() {
    let python_code = r#"
def shadow_test():
    x = 1
    if True:
        x = 2
        y = x + 1
    return x
"#;
    transpile_and_check(python_code, &["fn shadow_test"]);
}

#[test]
fn test_multiple_scope_levels() {
    let python_code = r#"
def deep_scopes():
    a = 1
    if True:
        b = 2
        if True:
            c = 3
            if True:
                d = 4
                result = a + b + c + d
    return result
"#;
    transpile_and_check(python_code, &["fn deep_scopes"]);
}

#[test]
fn test_loop_scoping() {
    let python_code = r#"
def loop_scope():
    total = 0
    for i in [1, 2, 3]:
        temp = i * 2
        total = total + temp
    return total
"#;
    transpile_and_check(python_code, &["fn loop_scope"]);
}

#[test]
fn test_parameter_scoping() {
    let python_code = r#"
def with_params(x: int, y: int) -> int:
    result = x + y
    return result
"#;
    transpile_and_check(python_code, &["fn with_params"]);
}

#[test]
fn test_union_type_processing() {
    let python_code = r#"
from typing import Union

def union_func(value: Union[int, str]) -> Union[int, str]:
    return value
"#;
    transpile_and_check(python_code, &["fn union_func"]);
}

#[test]
fn test_union_multiple_types() {
    let python_code = r#"
from typing import Union

def multi_union(value: Union[int, str, bool]) -> int:
    return 42
"#;
    transpile_and_check(python_code, &["fn multi_union"]);
}

#[test]
fn test_nested_union_types() {
    let python_code = r#"
from typing import Union

def nested_unions(
    a: Union[int, str],
    b: Union[float, bool]
) -> Union[int, str]:
    return a
"#;
    transpile_and_check(python_code, &["fn nested_unions"]);
}

#[test]
fn test_mutation_scope_invariants() {
    let python_code = r#"
def scope_invariant():
    x = 1
    y = x + 1
    if True:
        z = y + 1
        w = z + 1
    return x + y
"#;
    transpile_and_check(python_code, &["fn scope_invariant"]);
}

#[test]
fn test_union_enum_reuse() {
    let python_code = r#"
from typing import Union

def func1(value: Union[int, str]) -> Union[int, str]:
    return value

def func2(value: Union[int, str]) -> Union[int, str]:
    return value
"#;
    transpile_and_check(python_code, &["fn func1", "fn func2"]);
}

#[test]
fn test_empty_scope_stack() {
    let python_code = r#"
def simple():
    return 42
"#;
    transpile_and_check(python_code, &["fn simple", "42"]);
}

#[test]
fn test_complex_context_scenario() {
    let python_code = r#"
from typing import Union

def complex_context(value: Union[int, str]) -> int:
    result = 0
    if isinstance(value, int):
        temp = value * 2
        result = temp + 1
    else:
        temp = 42
        result = temp
    return result
"#;
    transpile_and_check(python_code, &["fn complex_context"]);
}

#[test]
fn test_mutable_variable_tracking() {
    let python_code = r#"
def mutable_tracking():
    x = 0
    x = x + 1
    x = x + 2
    return x
"#;
    transpile_and_check(python_code, &["fn mutable_tracking"]);
}

#[test]
fn test_exception_scope_bare_except() {
    let python_code = r#"
def try_bare_except():
    try:
        x = 1 / 0
    except:
        x = 0
    return x
"#;
    transpile_and_check(python_code, &["fn try_bare_except"]);
}

#[test]
fn test_exception_scope_specific_type() {
    let python_code = r#"
def try_specific():
    try:
        x = int("not a number")
    except ValueError:
        x = 0
    return x
"#;
    transpile_and_check(python_code, &["fn try_specific"]);
}

#[test]
fn test_exception_scope_multiple_types() {
    let python_code = r#"
def try_multiple():
    try:
        x = 1 / 0
        y = int("bad")
    except (ZeroDivisionError, ValueError):
        x = 0
    return x
"#;
    transpile_and_check(python_code, &["fn try_multiple"]);
}

#[test]
fn test_exception_scope_nested_try() {
    let python_code = r#"
def nested_try():
    try:
        x = 1
        try:
            y = 1 / 0
        except ZeroDivisionError:
            y = 0
    except ValueError:
        x = 0
    return x + y
"#;
    transpile_and_check(python_code, &["fn nested_try"]);
}

#[test]
fn test_exception_scope_with_finally() {
    let python_code = r#"
def try_finally():
    x = 0
    try:
        x = 1 / 0
    except:
        x = 1
    finally:
        x = x + 1
    return x
"#;
    transpile_and_check(python_code, &["fn try_finally"]);
}

#[test]
fn test_exception_scope_with_else() {
    let python_code = r#"
def try_else():
    try:
        x = 1
    except ValueError:
        x = 0
    else:
        x = x + 1
    return x
"#;
    transpile_and_check(python_code, &["fn try_else"]);
}

#[test]
fn test_exception_scope_raise_in_try() {
    let python_code = r#"
def raise_in_try(x: int):
    try:
        if x < 0:
            raise ValueError("negative")
        return x
    except ValueError:
        return 0
"#;
    transpile_and_check(python_code, &["fn raise_in_try"]);
}

#[test]
fn test_exception_scope_sequential_try() {
    let python_code = r#"
def sequential_try():
    try:
        x = 1 / 0
    except:
        x = 1

    try:
        y = int("bad")
    except:
        y = 2

    return x + y
"#;
    transpile_and_check(python_code, &["fn sequential_try"]);
}

#[test]
fn test_exception_scope_return_in_except() {
    let python_code = r#"
def return_in_except(x: int):
    try:
        result = 10 / x
    except ZeroDivisionError:
        return 0
    return result
"#;
    transpile_and_check(python_code, &["fn return_in_except"]);
}

#[test]
fn test_exception_scope_in_loop() {
    let python_code = r#"
def try_in_loop():
    result = 0
    for i in [1, 2, 0, 3]:
        try:
            result = result + 10 / i
        except ZeroDivisionError:
            result = result + 0
    return result
"#;
    transpile_and_check(python_code, &["fn try_in_loop"]);
}

#[test]
fn test_exception_scope_empty_except() {
    let python_code = r#"
def empty_except():
    try:
        x = 1 / 0
    except:
        pass
    return 0
"#;
    transpile_and_check(python_code, &["fn empty_except"]);
}

#[test]
fn test_exception_scope_multiple_except() {
    let python_code = r#"
def multiple_except():
    try:
        x = 1 / 0
    except ZeroDivisionError:
        x = 1
    except ValueError:
        x = 2
    except:
        x = 3
    return x
"#;
    transpile_and_check(python_code, &["fn multiple_except"]);
}

#[test]
fn test_mutation_exception_scope_stack() {
    let python_code = r#"
def nested_scopes():
    try:
        try:
            try:
                x = 1 / 0
            except ZeroDivisionError:
                x = 1
        except ValueError:
            x = 2
    except:
        x = 3
    return x
"#;
    transpile_and_check(python_code, &["fn nested_scopes"]);
}

#[test]
fn test_property_bare_except_catches_all() {
    let python_code = r#"
def bare_catches_all():
    try:
        x = 1 / 0
        y = int("bad")
        z = [].pop()
    except:
        return 0
    return 1
"#;
    transpile_and_check(python_code, &["fn bare_catches_all"]);
}

#[test]
fn test_integration_complex_exception_handling() {
    let python_code = r#"
def complex_exceptions(values: list[int]):
    total = 0
    for val in values:
        try:
            result = 100 / val
            try:
                formatted = str(result)
            except ValueError:
                formatted = "error"
        except ZeroDivisionError:
            result = 0
        finally:
            total = total + result
    return total
"#;
    transpile_and_check(python_code, &["fn complex_exceptions"]);
}
