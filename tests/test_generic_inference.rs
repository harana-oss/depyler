// HIR-level tests for generic inference that can't be expressed in TOML format
// Most generic inference tests have been migrated to toml/type-inference.toml

use depyler_core::{DepylerPipeline, hir::Type};

#[test]
fn test_union_type_hir() {
    let python_code = r#"
from typing import Union

def process_value(x: Union[int, str]) -> str:
    if isinstance(x, int):
        return str(x)
    else:
        return x
"#;

    let pipeline = DepylerPipeline::new();
    let hir = pipeline.parse_to_hir(python_code).unwrap();
    assert_eq!(hir.functions.len(), 1);

    let func = &hir.functions[0];
    match &func.params[0].ty {
        Type::Union(types) => {
            assert_eq!(types.len(), 2);
            assert!(types.contains(&Type::Int));
            assert!(types.contains(&Type::String));
        }
        _ => panic!("Expected Union type"),
    }
}

#[test]
fn test_generic_method_hir_representation() {
    let python_code = r#"
def test_hir():
    result = cast[int]("42")
    return result
"#;

    let pipeline = DepylerPipeline::new();
    let hir = pipeline.parse_to_hir(python_code).unwrap();
    assert_eq!(hir.functions.len(), 1);

    let func = &hir.functions[0];
    assert!(!func.body.is_empty());
}
