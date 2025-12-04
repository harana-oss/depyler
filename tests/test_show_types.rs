use crate::test_helpers::parse_to_hir;
use depyler_core::type_hints::TypeHintProvider;

#[test]
fn show_types_demo() {
    let python_code = r#"
def process(x, y):
    result = x + y
    return result * 2
"#;

    println!("\n=== Python Code ===");
    println!("{}", python_code);
    
    println!("\n=== After parse_to_hir (no inference) ===");
    let hir = parse_to_hir(python_code);
    
    for func in &hir.functions {
        println!("Function: {}", func.name);
        for param in &func.params {
            println!("  Param '{}': {:?}", param.name, param.ty);
        }
        println!("  Return type: {:?}", func.ret_type);
    }
    
    println!("\n=== After TypeHintProvider (usage-based inference) ===");
    let mut hint_provider = TypeHintProvider::new();
    for func in &hir.functions {
        let hints = hint_provider.analyze_function(func).unwrap();
        println!("Function: {}", func.name);
        if hints.is_empty() {
            println!("  (no hints generated)");
        }
        for hint in &hints {
            println!("  {:?} -> {:?} (confidence: {:?})", 
                hint.target, hint.suggested_type, hint.confidence);
            println!("    Reason: {}", hint.reason);
        }
    }
}
