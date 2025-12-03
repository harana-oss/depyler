mod test_helpers;

use test_helpers::transpile;

/// Helper function to transpile Python code without verifying compilation

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

    let rust = transpile(python);
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

    let rust = transpile(python);
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

    let rust = transpile(python);
    // Static attribute name generates direct field assignment without clone
    assert!(rust.contains("c.count = new_count"));
}

#[test]
fn test_setattr_string_variable_clones() {
    let python = r#"
class Person:
    name: str

def set_name(p: Person, new_name: str) -> None:
    setattr(p, "name", new_name)
"#;

    let rust = transpile(python);
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

    let rust = transpile(python);
    println!("Generated Rust code:\n{}", rust);
    // Object variables should be cloned
    assert!(rust.contains("new_inner.clone()"));
}

#[test]
fn test_setattr_dynamic_name_generates_set_field() {
    // Dynamic attribute names generate set_field() call
    let python = r#"
class Point:
    x: int
    y: int

def set_point_attr(p: Point, name: str, value: int) -> None:
    setattr(p, name, value)
"#;

    let rust = transpile(python);
    println!("Generated Rust code:\n{}", rust);
    // Should generate set_field() for dynamic attribute name
    assert!(rust.contains("._set_field("));
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

    let rust = transpile(python);
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

    let rust = transpile(python);
    println!("Generated Rust code:\n{}", rust);
    // Default is ignored in static Rust - just generate field access
    assert!(rust.contains("p.name"));
}

#[test]
fn test_getattr_dynamic_name_generates_get_field() {
    // Dynamic attribute names generate get_field() call
    let python = r#"
class Point:
    x: int
    y: int

def get_point_attr(p: Point, name: str) -> int:
    return getattr(p, name)
"#;

    let rust = transpile(python);
    println!("Generated Rust code:\n{}", rust);
    // Should generate get_field() for dynamic attribute name
    assert!(rust.contains("._get_field("));
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

    let rust = transpile(python);
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

    let rust = transpile(python);
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

    let rust = transpile(python);
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

    let rust = transpile(python);
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

    let rust = transpile(python);
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

    let rust = transpile(python);
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

    let rust = transpile(python);
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

    let rust = transpile(python);
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

    let rust = transpile(python);
    println!("Generated Rust code:\n{}", rust);
    // state should be &mut State since setattr mutates it
    assert!(rust.contains("state: &mut State"));
    // config should be cloned when assigning
    assert!(rust.contains("config.clone()"));
}

// ============================================================================
// getattr/setattr with f-string attribute names
// ============================================================================

#[test]
fn test_getattr_fstring_attribute() {
    // f-string attribute names generate get_field() call
    let python = r#"
def get_team_stat(stats: dict, team: str) -> int:
    return getattr(stats, f'{team}_score')
"#;

    let rust = transpile(python);
    println!("Generated Rust code:\n{}", rust);
    // Should generate format! for the key and get_field() access
    assert!(rust.contains("format!"));
    assert!(rust.contains("._get_field("));
}

#[test]
fn test_getattr_fstring_with_default() {
    // f-string attribute names with default value
    let python = r#"
def get_team_stat_or_default(stats: dict, team: str) -> int:
    return getattr(stats, f'{team}_score', 0)
"#;

    let rust = transpile(python);
    println!("Generated Rust code:\n{}", rust);
    // Should generate format! for the key and get_field() with unwrap_or
    assert!(rust.contains("format!"));
    assert!(rust.contains("._get_field("));
    assert!(rust.contains("unwrap_or"));
}

#[test]
fn test_getattr_fstring_in_for_loop() {
    let python = r#"
def process_stats(state: State, one: str) -> int:
    total: int = 0
    for stats in getattr(state, f'{one}_two'):
        total = total + stats
    return total
"#;

    let rust = transpile(python);
    println!("Generated Rust code:\n{}", rust);
    // Dynamic f-string attribute name uses get_field() for runtime resolution
    // Cannot resolve runtime variable `one` to static field name
    assert!(rust.contains("._get_field(&format!"));
}

#[test]
fn test_setattr_fstring_attribute() {
    // f-string attribute names generate set_field() call
    let python = r#"
def set_team_stat(stats: dict, team: str, value: int) -> None:
    setattr(stats, f'{team}_score', value)
"#;

    let rust = transpile(python);
    println!("Generated Rust code:\n{}", rust);
    // Should generate format! for the key and set_field()
    assert!(rust.contains("format!"));
    assert!(rust.contains("._set_field("));
}
