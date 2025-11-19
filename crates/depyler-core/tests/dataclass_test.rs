//! Tests for Python dataclass (@dataclass decorator) support
//!
//! This test suite verifies that Python's @dataclass decorator is properly
//! transpiled to Rust structs with appropriate derives and constructors.

use depyler_core::DepylerPipeline;

#[test]
fn test_simple_dataclass() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class Point:
    x: int
    y: int
"#;

    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(
        result.is_ok(),
        "Transpilation failed: {:?}",
        result.as_ref().err()
    );

    let rust_code = result.unwrap();

    // Should have Debug, Clone, PartialEq derives for dataclasses
    assert!(
        rust_code.contains("#[derive(Debug, Clone, PartialEq)]"),
        "Dataclass should have Debug, Clone, PartialEq derives.\nGot:\n{}",
        rust_code
    );

    // Should have struct definition
    assert!(
        rust_code.contains("struct Point"),
        "Should have Point struct.\nGot:\n{}",
        rust_code
    );

    // Should have new() constructor
    assert!(
        rust_code.contains("pub fn new"),
        "Dataclass should have new() constructor.\nGot:\n{}",
        rust_code
    );

    // Should NOT have the TODO import comment
    assert!(
        !rust_code.contains("TODO: Map Python module 'dataclasses'"),
        "Should not have TODO comment for dataclasses import.\nGot:\n{}",
        rust_code
    );
}

#[test]
fn test_dataclass_with_methods() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class Rectangle:
    width: int
    height: int

    def area(self) -> int:
        return self.width * self.height

    def perimeter(self) -> int:
        return 2 * (self.width + self.height)
"#;

    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(
        result.is_ok(),
        "Transpilation failed: {:?}",
        result.as_ref().err()
    );

    let rust_code = result.unwrap();

    // Should have dataclass derives
    assert!(
        rust_code.contains("#[derive(Debug, Clone, PartialEq)]"),
        "Dataclass should have PartialEq derive.\nGot:\n{}",
        rust_code
    );

    // Should have methods
    assert!(
        rust_code.contains("fn area"),
        "Should have area method.\nGot:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("fn perimeter"),
        "Should have perimeter method.\nGot:\n{}",
        rust_code
    );
}

#[test]
fn test_dataclass_with_staticmethod() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class Point:
    x: int
    y: int

    @staticmethod
    def origin():
        return Point(0, 0)
"#;

    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(
        result.is_ok(),
        "Transpilation failed: {:?}",
        result.as_ref().err()
    );

    let rust_code = result.unwrap();

    // Static method should not have &self parameter
    assert!(
        rust_code.contains("pub fn origin"),
        "Should have origin static method.\nGot:\n{}",
        rust_code
    );

    // Should have return type Self
    assert!(
        rust_code.contains("-> Self"),
        "Static method should return Self.\nGot:\n{}",
        rust_code
    );
}

#[test]
fn test_dataclass_with_classmethod() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class Point:
    x: int
    y: int

    @classmethod
    def from_tuple(cls, coords: tuple):
        return cls(coords[0], coords[1])
"#;

    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(
        result.is_ok(),
        "Transpilation failed: {:?}",
        result.as_ref().err()
    );

    let rust_code = result.unwrap();

    // Classmethod should not have &self parameter
    assert!(
        rust_code.contains("pub fn from_tuple"),
        "Should have from_tuple classmethod.\nGot:\n{}",
        rust_code
    );

    // Should have return type Self
    assert!(
        rust_code.contains("-> Self"),
        "Classmethod should return Self.\nGot:\n{}",
        rust_code
    );
}

#[test]
fn test_dataclass_with_property() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class Point:
    x: int
    y: int

    @property
    def magnitude(self) -> float:
        return (self.x * self.x + self.y * self.y) ** 0.5
"#;

    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(
        result.is_ok(),
        "Transpilation failed: {:?}",
        result.as_ref().err()
    );

    let rust_code = result.unwrap();

    // Property should be converted to a method
    assert!(
        rust_code.contains("pub fn magnitude"),
        "Property should become a method.\nGot:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("&self"),
        "Property should have &self parameter.\nGot:\n{}",
        rust_code
    );
}

#[test]
fn test_dataclass_with_default_values() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class Config:
    name: str
    debug: bool = False
    timeout: int = 30
"#;

    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(
        result.is_ok(),
        "Transpilation failed: {:?}",
        result.as_ref().err()
    );

    let rust_code = result.unwrap();

    // Should have struct with all fields
    assert!(
        rust_code.contains("struct Config"),
        "Should have Config struct.\nGot:\n{}",
        rust_code
    );

    // Should have derives
    assert!(
        rust_code.contains("#[derive(Debug, Clone, PartialEq)]"),
        "Should have dataclass derives.\nGot:\n{}",
        rust_code
    );
}

#[test]
fn test_dataclass_with_multiple_types() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class Person:
    name: str
    age: int
    height: float
    is_student: bool
"#;

    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(
        result.is_ok(),
        "Transpilation failed: {:?}",
        result.as_ref().err()
    );

    let rust_code = result.unwrap();

    assert!(
        rust_code.contains("struct Person"),
        "Should have Person struct.\nGot:\n{}",
        rust_code
    );

    // Fields should be present (exact types may vary with type mapping)
    let has_fields = rust_code.contains("name")
        && rust_code.contains("age")
        && rust_code.contains("height")
        && rust_code.contains("is_student");

    assert!(has_fields, "Should have all fields.\nGot:\n{}", rust_code);
}

#[test]
fn test_dataclass_no_explicit_init() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class Coordinate:
    x: float
    y: float
    z: float
"#;

    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(
        result.is_ok(),
        "Transpilation failed: {:?}",
        result.as_ref().err()
    );

    let rust_code = result.unwrap();

    // Dataclass without explicit __init__ should still get a new() constructor
    assert!(
        rust_code.contains("pub fn new"),
        "Dataclass should have generated new() constructor.\nGot:\n{}",
        rust_code
    );

    // Should have struct
    assert!(
        rust_code.contains("struct Coordinate"),
        "Should have Coordinate struct.\nGot:\n{}",
        rust_code
    );
}

#[test]
fn test_regular_class_vs_dataclass() {
    // Test that regular classes don't get PartialEq derive
    let python_regular = r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y
"#;

    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python_regular);
    assert!(result.is_ok());

    let rust_code = result.unwrap();

    // Regular class should have Debug, Clone but NOT PartialEq
    assert!(
        rust_code.contains("#[derive(Debug, Clone)]"),
        "Regular class should have Debug, Clone.\nGot:\n{}",
        rust_code
    );

    assert!(
        !rust_code.contains("PartialEq"),
        "Regular class should NOT have PartialEq.\nGot:\n{}",
        rust_code
    );
}

#[test]
fn test_dataclass_empty_body() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class EmptyData:
    value: int
"#;

    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(
        result.is_ok(),
        "Transpilation failed: {:?}",
        result.as_ref().err()
    );

    let rust_code = result.unwrap();

    // Even with just field declarations, should work
    assert!(
        rust_code.contains("struct EmptyData"),
        "Should have EmptyData struct.\nGot:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("pub fn new"),
        "Should have new() constructor.\nGot:\n{}",
        rust_code
    );
}
