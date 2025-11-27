// Test script to manually check what type inference produces
use depyler_core::dataflow::infer_python;

fn main() {
    let test_cases = vec![
        (
            "int_cast",
            r#"
def test():
    y = 3.14
    x = int(y)
    return x
"#,
        ),
        (
            "abs_int",
            r#"
def test():
    h = -42
    g = abs(h)
    return g
"#,
        ),
        (
            "abs_float",
            r#"
def test():
    h = -3.14
    g = abs(h)
    return g
"#,
        ),
        (
            "str_cast",
            r#"
def test():
    y = 42
    x = str(y)
    return x
"#,
        ),
    ];

    for (name, code) in test_cases {
        println!("\n=== Test: {} ===", name);
        match infer_python(code) {
            Ok(results) => {
                for (func_name, types) in results {
                    println!("Function: {}", func_name);
                    println!("  Return type: {:?}", types.return_type);
                    for (var, ty) in types.all_variables() {
                        println!("  {}: {:?}", var, ty);
                    }
                    println!("  Iterations: {}", types.iterations);
                    println!("  Complete: {}", types.is_complete);
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
