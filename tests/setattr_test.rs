use depyler_core::DepylerPipeline;

/// Helper function to transpile Python code without verifying compilation
fn transpile_only(python: &str) -> Result<String, Box<dyn std::error::Error>> {
    let pipeline = DepylerPipeline::new();
    let rust_code = pipeline.transpile(python)?;
    Ok(rust_code)
}

// ============================================================================
// setattr Tests
// ============================================================================

#[test]
fn test_setattr_literal_field() {
    let python = r#"
class Config:
    value: int

def set_config_value(c: Config) -> None:
    setattr(c, "value", 42)
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // When field name is a literal, it should generate direct field assignment
    assert!(rust.contains("c.value = 42"));
}

#[test]
fn test_setattr_string_value() {
    let python = r#"
class Person:
    name: str

def set_name(p: Person) -> None:
    setattr(p, "name", "Alice")
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // String values should be converted to String (not &str)
    assert!(rust.contains(r#"p.name = "Alice".to_string()"#) || rust.contains(r#"p.name = String::from("Alice")"#));
}

#[test]
fn test_setattr_variable_value() {
    let python = r#"
class Counter:
    count: int

def set_count(c: Counter, new_count: int) -> None:
    setattr(c, "count", new_count)
"#;

    let rust = transpile_only(python).unwrap();
    assert!(rust.contains("c.count = new_count"));
}

#[test]
fn test_setattr_dynamic_name_errors() {
    // Dynamic attribute names cannot be transpiled to static Rust
    let python = r#"
class Point:
    x: int
    y: int

def set_point_attr(p: Point, name: str, value: int) -> None:
    setattr(p, name, value)
"#;

    let result = transpile_only(python);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("string literal"));
}

// ============================================================================
// getattr Tests
// ============================================================================

#[test]
fn test_getattr_literal_field() {
    let python = r#"
class Config:
    value: int

def get_config_value(c: Config) -> int:
    return getattr(c, "value")
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // When field name is a literal, it should generate direct field access
    assert!(rust.contains("c.value"));
}

#[test]
fn test_getattr_with_default() {
    let python = r#"
class Person:
    name: str

def get_name(p: Person) -> str:
    return getattr(p, "name", "Unknown")
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // Default is ignored in static Rust - just generate field access
    assert!(rust.contains("p.name"));
}

#[test]
fn test_getattr_dynamic_name_errors() {
    // Dynamic attribute names cannot be transpiled to static Rust
    let python = r#"
class Point:
    x: int
    y: int

def get_point_attr(p: Point, name: str) -> int:
    return getattr(p, name)
"#;

    let result = transpile_only(python);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("string literal"));
}
