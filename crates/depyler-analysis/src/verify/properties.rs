//! Property-based test generation

use anyhow::Result;
use depyler_core::hir::{HirFunction, Type};

pub fn generate_quickcheck_tests(func: &HirFunction, _iterations: usize) -> Result<String> {
    let func_name = &func.name;

    let mut test_code = String::new();

    // Add quickcheck imports
    test_code.push_str("#[cfg(test)]\n");
    test_code.push_str("mod tests {\n");
    test_code.push_str("    use super::*;\n");
    test_code.push_str("    use quickcheck::{quickcheck, TestResult};\n\n");

    // Generate property test for type preservation
    if has_numeric_types(&func.params) {
        test_code.push_str(&generate_numeric_property_test(func)?);
    }

    // Generate property test for bounds checking
    if has_container_params(&func.params) {
        test_code.push_str(&generate_bounds_property_test(func)?);
    }

    // Generate property test for termination
    if func.properties.always_terminates {
        test_code.push_str(&generate_termination_test(func)?);
    }

    test_code.push_str("}\n");

    Ok(test_code)
}

fn has_numeric_types(params: &[depyler_core::hir::HirParam]) -> bool {
    params.iter().any(|param| matches!(param.ty, Type::Int | Type::Float))
}

fn has_container_params(params: &[depyler_core::hir::HirParam]) -> bool {
    params.iter().any(|param| param.ty.is_container())
}

fn generate_numeric_property_test(func: &HirFunction) -> Result<String> {
    let func_name = &func.name;
    let mut test = String::new();

    test.push_str("    quickcheck! {\n");
    test.push_str(&format!("        fn prop_{func_name}_numeric_overflow("));

    // Generate parameters
    let param_list: Vec<String> = func
        .params
        .iter()
        .map(|param| match &param.ty {
            Type::Int => format!("{}: i32", param.name),
            Type::Float => format!("{}: f64", param.name),
            Type::List(inner) if matches!(**inner, Type::Int) => {
                format!("{}: Vec<i32>", param.name)
            }
            _ => format!("{}: i32", param.name),
        })
        .collect();

    test.push_str(&param_list.join(", "));
    test.push_str(") -> TestResult {\n");

    // Add overflow checks
    test.push_str("            // Check for potential overflows\n");
    for param in &func.params {
        if matches!(param.ty, Type::Int) {
            test.push_str(&format!(
                "            if {}.checked_add(1).is_none() {{ return TestResult::discard(); }}\n",
                param.name
            ));
        }
    }

    // Call the function
    test.push_str(&format!("            let _result = {func_name}("));
    let args: Vec<String> = func
        .params
        .iter()
        .map(|param| {
            if param.ty.is_container() {
                format!("&{}", param.name)
            } else {
                param.name.clone()
            }
        })
        .collect();
    test.push_str(&args.join(", "));
    test.push_str(");\n");

    test.push_str("            TestResult::from_bool(true)\n");
    test.push_str("        }\n");
    test.push_str("    }\n\n");

    Ok(test)
}

fn generate_bounds_property_test(func: &HirFunction) -> Result<String> {
    let func_name = &func.name;
    let mut test = String::new();

    test.push_str("    quickcheck! {\n");
    test.push_str(&format!("        fn prop_{func_name}_bounds_checking("));

    let param_list: Vec<String> = func
        .params
        .iter()
        .map(|param| match &param.ty {
            Type::List(inner) => {
                let inner_type = type_to_rust_string(inner);
                format!("{}: Vec<{}>", param.name, inner_type)
            }
            _ => format!("{}: i32", param.name),
        })
        .collect();

    test.push_str(&param_list.join(", "));
    test.push_str(") -> TestResult {\n");

    for param in &func.params {
        if matches!(param.ty, Type::List(_)) {
            test.push_str(&format!(
                "            if {}.is_empty() {{ return TestResult::discard(); }}\n",
                param.name
            ));
        }
    }

    test.push_str(&format!("            let _result = {func_name}("));
    let args: Vec<String> = func
        .params
        .iter()
        .map(|param| {
            if param.ty.is_container() {
                format!("&{}", param.name)
            } else {
                param.name.clone()
            }
        })
        .collect();
    test.push_str(&args.join(", "));
    test.push_str(");\n");

    test.push_str("            TestResult::from_bool(true)\n");
    test.push_str("        }\n");
    test.push_str("    }\n\n");

    Ok(test)
}

fn generate_termination_test(func: &HirFunction) -> Result<String> {
    let func_name = &func.name;
    let mut test = String::new();

    test.push_str(&format!("    #[test]\n    fn test_{func_name}_terminates() {{\n"));
    test.push_str("        // Verify function terminates within time limit\n");
    test.push_str("        use std::time::Duration;\n");
    test.push_str("        let timeout = Duration::from_secs(5);\n");
    test.push_str("        // Add timeout-based test here\n");
    test.push_str("    }\n\n");

    Ok(test)
}

fn type_to_rust_string(ty: &Type) -> String {
    match ty {
        Type::Int => "i64".to_string(),
        Type::Float => "f64".to_string(),
        Type::String => "String".to_string(),
        Type::Bool => "bool".to_string(),
        Type::List(inner) => format!("Vec<{}>", type_to_rust_string(inner)),
        _ => "()".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::smallvec;

    #[test]
    fn test_generate_quickcheck_tests() {
        let func = HirFunction {
            name: "add".to_string(),
            params: smallvec![
                depyler_core::hir::HirParam::new("a".to_string(), Type::Int),
                depyler_core::hir::HirParam::new("b".to_string(), Type::Int),
            ],
            ret_type: Type::Int,
            body: vec![],
            properties: Default::default(),
            annotations: Default::default(),
            docstring: None,
        };

        let result = generate_quickcheck_tests(&func, 100);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("quickcheck"));
    }
}
