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
    // Integer variables don't need clone
    assert!(rust.contains("c.count = new_count"));
    assert!(!rust.contains("clone"));
}

#[test]
fn test_setattr_string_variable_clones() {
    let python = r#"
class Person:
    name: str

def set_name(p: Person, new_name: str) -> None:
    setattr(p, "name", new_name)
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // String variables should be cloned
    assert!(rust.contains("new_name.clone()"));
}

#[test]
fn test_setattr_object_variable_clones() {
    let python = r#"
class Inner:
    value: int

class Outer:
    inner: Inner

def set_inner(o: Outer, new_inner: Inner) -> None:
    setattr(o, "inner", new_inner)
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // Object variables should be cloned
    assert!(rust.contains("new_inner.clone()"));
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

// ============================================================================
// setattr in Lambda/Closure Tests
// ============================================================================

#[test]
fn test_setattr_in_lambda() {
    let python = r#"
class State:
    temp: int
    result: int

def update_state(s: State) -> None:
    setattr(s, "temp", getattr(s, "result"))
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // Should generate: s.temp = s.result
    assert!(rust.contains("s.temp = s.result"));
}

#[test]
fn test_setattr_copy_string_field() {
    let python = r#"
class Data:
    source: str
    target: str

def copy_field(d: Data) -> None:
    setattr(d, "target", getattr(d, "source"))
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // Should generate: d.target = d.source (with appropriate cloning)
    assert!(rust.contains("d.target = d.source"));
}

#[test]
fn test_getattr_in_lambda() {
    let python = r#"
class Point:
    x: int
    y: int

def get_coords(points: list) -> list:
    return list(map(lambda p: getattr(p, "x"), points))
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // Lambda should access p.x
    assert!(rust.contains("p.x"));
}

#[test]
fn test_getattr_in_filter_lambda() {
    let python = r#"
class Item:
    active: bool

def get_active(items: list) -> list:
    return list(filter(lambda i: getattr(i, "active"), items))
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // Lambda should access i.active
    assert!(rust.contains("i.active"));
}

#[test]
fn test_setattr_string_in_lambda() {
    let python = r#"
class Person:
    name: str

def set_names(people: list, new_name: str) -> None:
    for p in people:
        setattr(p, "name", new_name)
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // String parameter should be cloned when used in setattr
    assert!(rust.contains("new_name.clone()") || rust.contains("p.name ="));
}

#[test]
fn test_setattr_struct_in_loop() {
    let python = r#"
class Config:
    value: int

class Container:
    config: Config

def update_configs(containers: list, new_config: Config) -> None:
    for c in containers:
        setattr(c, "config", new_config)
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // Struct parameter should be cloned when used in setattr within a loop
    assert!(rust.contains("new_config.clone()"));
}

#[test]
fn test_setattr_struct_from_getattr_in_loop() {
    let python = r#"
class Inner:
    data: int

class Source:
    inner: Inner

class Target:
    inner: Inner

def copy_inner(sources: list, targets: list) -> None:
    for i in range(len(sources)):
        setattr(targets[i], "inner", getattr(sources[i], "inner"))
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // Should generate field access for both getattr and setattr
    assert!(rust.contains(".inner"));
}

#[test]
fn test_setattr_struct_value_in_lambda() {
    // Test: update(lambda s: setattr(s, "config", new_config)) where new_config is a struct
    let python = r#"
class Config:
    value: str

class State:
    value: str

def apply_config(states: list, new_config: Config) -> list:
    return list(map(lambda s: setattr(s, "value", new_config.value), states))
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // Lambda should set s.config with the struct value cloned
    assert!(rust.contains("s.value = new_config.value.clone()"))
}

#[test]
fn test_setattr_mut_struct_ref_in_lambda() {
    // Test: setattr(state, "config", config) where state should be &mut State
    let python = r#"
class Config:
    value: int

class State:
    config: Config

def update_with_config(state: State, config: Config) -> None:
    setattr(state, "config", config)
"#;

    let rust = transpile_only(python).unwrap();
    println!("Generated Rust code:\n{}", rust);
    // state should be &mut State since setattr mutates it
    assert!(rust.contains("state: &mut State"));
    // config should be cloned when assigning
    assert!(rust.contains("config.clone()"));
}
