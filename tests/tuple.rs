use depyler_core::DepylerPipeline;

fn transpile_only(python: &str) -> Result<String, Box<dyn std::error::Error>> {
    let pipeline = DepylerPipeline::new();
    let rust_code = pipeline.transpile(python)?;
    Ok(rust_code)
}

#[test]
fn test_string_variable_in_tuple() {
    let python = r#"
def check_location(location: str) -> bool:
    return location in ("Home", "Away")
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    assert!(rust.contains("[\"Home\".to_string(), \"Away\".to_string()].contains(&location)"));
}

#[test]
fn test_string_literal_in_tuple() {
    let python = r#"
def check_location() -> bool:
    return "Home" in ("Home", "Away")
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    assert!(rust.contains("[\"Home\".to_string(), \"Away\".to_string()].contains(\"Home\".to_string())"));
}
