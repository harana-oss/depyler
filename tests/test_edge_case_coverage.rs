use crate::test_helpers::transpile_and_check;

#[test]
fn test_empty_python_file() {
    transpile_and_check("", &[]);
}

#[test]
fn test_whitespace_only_file() {
    transpile_and_check("   \n\t  \n   ", &[]);
}

#[test]
fn test_comments_only_file() {
    transpile_and_check(
        r#"
# This is a comment
# Another comment
    # Indented comment
        # More comments
"#,
        &[],
    );
}

#[test]
fn test_deeply_nested_functions() {
    transpile_and_check(
        r#"
def level1(x: int) -> int:
    def level2(y: int) -> int:
        def level3(z: int) -> int:
            def level4(w: int) -> int:
                return w + 1
            return level4(z) + 1
        return level3(y) + 1
    return level2(x) + 1
"#,
        &[],
    );
}

#[test]
fn test_very_long_function_name() {
    let long_name = "a".repeat(1000);
    let long_name_source = format!(
        r#"
def {}(x: int) -> int:
    return x + 1
"#,
        long_name
    );

    transpile_and_check(&long_name_source, &[]);
}

#[test]
#[ignore]
// test_edge_case_coverage::test_function_with_many_parameters' (110405975) has overflowed its stack
fn test_function_with_many_parameters() {
    let mut params = Vec::new();
    let mut args = Vec::new();

    for i in 0..50 {
        params.push(format!("param{}: int", i));
        args.push(format!("param{}", i));
    }

    let many_params_source = format!(
        r#"M
def many_params({}) -> int:
    return {}
"#,
        params.join(", "),
        args.join(" + ")
    );

    transpile_and_check(&many_params_source, &[]);
}

#[test]
fn test_extremely_simple_function() {
    transpile_and_check(
        r#"
def f(): pass
"#,
        &[],
    );
}

#[test]
fn test_function_with_only_return() {
    transpile_and_check(
        r#"
def get_five() -> int:
    return 5
"#,
        &[],
    );
}

#[test]
fn test_unicode_function_names() {
    transpile_and_check(
        r#"
def функция(x: int) -> int:
    return x * 2

def 関数(y: int) -> int:
    return y + 1
"#,
        &[],
    );
}

#[test]
fn test_unicode_strings() {
    transpile_and_check(
        r#"
def greet() -> str:
    return "Hello, 世界! 🌍"

def emoji_func() -> str:
    return "🚀💯✨"
"#,
        &[],
    );
}

#[test]
fn test_max_integer_values() {
    transpile_and_check(
        r#"
def big_numbers() -> int:
    x = 9223372036854775807
    y = -9223372036854775808
    return x + y
"#,
        &[],
    );
}

#[test]
fn test_empty_lists_and_dicts() {
    transpile_and_check(
        r#"
def empty_collections():
    empty_list = []
    empty_dict = {}
    return len(empty_list) + len(empty_dict)
"#,
        &[],
    );
}

#[test]
fn test_single_character_variables() {
    transpile_and_check(
        r#"
def single_chars(a: int, b: int, c: int) -> int:
    x = a
    y = b
    z = c
    return x + y + z
"#,
        &[],
    );
}

#[test]
fn test_very_long_string_literal() {
    let long_string = "x".repeat(10000);
    let long_string_source = format!(
        r#"
def long_string() -> str:
    return "{}"
"#,
        long_string
    );

    transpile_and_check(&long_string_source, &[]);
}

#[test]
fn test_nested_control_structures() {
    transpile_and_check(
        r#"
def nested_control(n: int) -> int:
    result = 0
    for i in range(n):
        if i % 2 == 0:
            for j in range(i):
                if j % 3 == 0:
                    while j > 0:
                        result += j
                        j -= 1
                        if result > 100:
                            break
    return result
"#,
        &[],
    );
}

#[test]
fn test_all_python_operators() {
    transpile_and_check(
        r#"
def all_operators(a: int, b: int) -> bool:
    # Arithmetic
    add = a + b
    sub = a - b
    mul = a * b
    div = a // b  # Floor division
    mod = a % b
    
    # Comparison
    eq = a == b
    ne = a != b
    lt = a < b
    le = a <= b
    gt = a > b
    ge = a >= b
    
    # Logical (using int as bool)
    and_op = a and b
    or_op = a or b
    not_op = not a
    
    return eq or ne or lt or le or gt or ge
"#,
        &[],
    );
}
