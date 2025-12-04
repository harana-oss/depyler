//! Consolidated tests for Python class transpilation
//!
//! This file consolidates:
//! - class_basic_test.rs (Phase 1: Simple classes with __init__)
//! - class_methods_test.rs (Phase 2: Instance methods)
//! - class_attributes_test.rs (Phase 3: Class-level attributes)
//! - multiple_classes_test.rs (Phase 4: Multiple classes)
//! - staticmethod_test.rs (Phase 5: @staticmethod decorator)
//! - classmethod_test.rs (Phase 6: @classmethod decorator)
//! - simple_method_test.rs (Basic method tests)

mod test_helpers;

use depyler_core::DepylerPipeline;

// ============================================================================
// Phase 1: Simple Classes with __init__
// ============================================================================

mod basic_classes {
    use super::*;

    #[test]
    fn test_simple_class_with_init() {
        let python = r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Point"),
            "Should generate struct Point.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("x:") && rust_code.contains("y:"),
            "Should have x and y fields.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("impl Point"),
            "Should generate impl block.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn new"),
            "Should have new() constructor.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_class_with_typed_fields() {
        let python = r#"
class Rectangle:
    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Rectangle"),
            "Should generate struct Rectangle.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("width") && rust_code.contains("height"),
            "Should have width and height fields.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_class_with_default_parameters() {
        let python = r#"
class Counter:
    def __init__(self, start: int = 0):
        self.value = start
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Counter"),
            "Should generate struct Counter.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("value"),
            "Should have value field.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_empty_class() {
        let python = r#"
class Empty:
    pass
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Empty"),
            "Should generate struct Empty.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_class_with_multiple_fields() {
        let python = r#"
class Student:
    def __init__(self, name: str, age: int, grade: float, active: bool):
        self.name = name
        self.age = age
        self.grade = grade
        self.active = active
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Student"),
            "Should generate struct Student.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("name")
                && rust_code.contains("age")
                && rust_code.contains("grade")
                && rust_code.contains("active"),
            "Should have all 4 fields.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_class_instantiation() {
        let python = r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

def create_point() -> Point:
    p = Point(10, 20)
    return p
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("Point::new") || rust_code.contains("Point {"),
            "Should create Point instance.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_field_access() {
        let python = r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

def get_x(p: Point) -> int:
    return p.x
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("p.x") || rust_code.contains("p .x"),
            "Should access field p.x.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_class_field_mutation() {
        let python = r#"
class Counter:
    def __init__(self, value: int):
        self.value = value

def increment(c: Counter) -> int:
    c.value = c.value + 1
    return c.value
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("c.value") || rust_code.contains("c .value"),
            "Should access and mutate c.value.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_class_with_computed_field() {
        let python = r#"
class Rectangle:
    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height
        self.area = width * height
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Rectangle"),
            "Should generate struct Rectangle.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("area"),
            "Should have computed area field.\nGot:\n{}",
            rust_code
        );
    }
}

// ============================================================================
// Phase 2: Instance Methods
// ============================================================================

mod instance_methods {
    use super::*;

    #[test]
    fn test_simple_instance_method() {
        let python = r#"
class Counter:
    def __init__(self, value: int):
        self.value = value

    def get_value(self) -> int:
        return self.value
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("impl Counter"),
            "Should have impl Counter block.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn get_value"),
            "Should have get_value method.\nGot:\n{}",
            rust_code
        );

        let has_self_ref = rust_code.contains("&self") || rust_code.contains("& self");
        assert!(has_self_ref, "Method should take &self parameter.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_instance_method_accessing_fields() {
        let python = r#"
class Rectangle:
    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height

    def area(self) -> int:
        return self.width * self.height
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn area"),
            "Should have area method.\nGot:\n{}",
            rust_code
        );

        let has_width = rust_code.contains("self.width") || rust_code.contains("self .width");
        let has_height = rust_code.contains("self.height") || rust_code.contains("self .height");
        assert!(
            has_width && has_height,
            "Should access self.width and self.height.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_instance_method_modifying_self() {
        let python = r#"
class Counter:
    def __init__(self, value: int):
        self.value = value

    def increment(self) -> None:
        self.value = self.value + 1
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn increment"),
            "Should have increment method.\nGot:\n{}",
            rust_code
        );

        let has_mut_self = rust_code.contains("&mut self") || rust_code.contains("& mut self");
        assert!(
            has_mut_self,
            "Mutating method should take &mut self.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_instance_method_with_parameters() {
        let python = r#"
class Calculator:
    def __init__(self, initial: int):
        self.value = initial

    def add(self, amount: int) -> int:
        return self.value + amount
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn add"),
            "Should have add method.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("amount"),
            "Should have amount parameter.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_instance_method_calling_another_method() {
        let python = r#"
class Circle:
    def __init__(self, radius: float):
        self.radius = radius

    def diameter(self) -> float:
        return self.radius * 2.0

    def circumference(self) -> float:
        return self.diameter() * 3.14159
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn diameter"),
            "Should have diameter method.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn circumference"),
            "Should have circumference method.\nGot:\n{}",
            rust_code
        );

        let calls_diameter = rust_code.contains("self.diameter()") || rust_code.contains("self .diameter");
        assert!(calls_diameter, "Should call self.diameter().\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_multiple_instance_methods() {
        let python = r#"
class BankAccount:
    def __init__(self, balance: int):
        self.balance = balance

    def deposit(self, amount: int) -> None:
        self.balance = self.balance + amount

    def withdraw(self, amount: int) -> None:
        self.balance = self.balance - amount

    def get_balance(self) -> int:
        return self.balance
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn deposit"),
            "Should have deposit method.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn withdraw"),
            "Should have withdraw method.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn get_balance"),
            "Should have get_balance method.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_simple_method_call() {
        let pipeline = DepylerPipeline::new();

        let python_code = r#"
def simple_append():
    numbers = [1, 2, 3]
    numbers.append(4)
    return numbers
"#;

        let result = pipeline.transpile(python_code);
        if let Ok(rust_code) = result {
            assert!(rust_code.contains("push"), "Should use push for append");
        }
    }
}

// ============================================================================
// Phase 3: Class-Level Attributes
// ============================================================================

mod class_attributes {
    use super::*;

    #[test]
    fn test_simple_class_constant() {
        let python = r#"
class Config:
    MAX_SIZE: int = 100

    def __init__(self, size: int):
        self.size = size
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Config"),
            "Should have Config struct.\nGot:\n{}",
            rust_code
        );

        let has_const = rust_code.contains("const MAX_SIZE")
            || rust_code.contains("const max_size")
            || rust_code.contains("MAX_SIZE: i32 = 100");
        assert!(has_const, "Should have MAX_SIZE constant.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_class_attribute_access_via_classname() {
        let python = r#"
class Math:
    PI: float = 3.14159

    def get_pi(self) -> float:
        return Math.PI
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Math"),
            "Should have Math struct.\nGot:\n{}",
            rust_code
        );

        let has_pi = rust_code.contains("PI") || rust_code.contains("pi");
        assert!(has_pi, "Should have PI constant.\nGot:\n{}", rust_code);
        assert!(
            rust_code.contains("fn get_pi"),
            "Should have get_pi method.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_multiple_class_attributes() {
        let python = r#"
class Constants:
    WIDTH: int = 800
    HEIGHT: int = 600
    TITLE: str = "Game"

    def __init__(self):
        self.width = Constants.WIDTH
        self.height = Constants.HEIGHT
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        let has_width = rust_code.contains("WIDTH") || rust_code.contains("width");
        let has_height = rust_code.contains("HEIGHT") || rust_code.contains("height");
        let has_title = rust_code.contains("TITLE") || rust_code.contains("title");

        assert!(has_width, "Should have WIDTH constant.\nGot:\n{}", rust_code);
        assert!(has_height, "Should have HEIGHT constant.\nGot:\n{}", rust_code);
        assert!(has_title, "Should have TITLE constant.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_class_attribute_used_in_method() {
        let python = r#"
class Circle:
    PI: float = 3.14159

    def __init__(self, radius: float):
        self.radius = radius

    def area(self) -> float:
        return Circle.PI * self.radius * self.radius
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Circle"),
            "Should have Circle struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn area"),
            "Should have area method.\nGot:\n{}",
            rust_code
        );

        let has_pi = rust_code.contains("PI") || rust_code.contains("pi");
        assert!(has_pi, "Should have PI constant.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_mix_class_and_instance_attributes() {
        let python = r#"
class Car:
    WHEELS: int = 4

    def __init__(self, color: str):
        self.color = color
        self.wheels = Car.WHEELS

    def get_wheels(self) -> int:
        return self.wheels
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Car"),
            "Should have Car struct.\nGot:\n{}",
            rust_code
        );

        let has_wheels = rust_code.contains("WHEELS") || rust_code.contains("wheels");
        assert!(has_wheels, "Should have WHEELS constant or field.\nGot:\n{}", rust_code);

        let has_color = rust_code.contains("color");
        assert!(has_color, "Should have color field.\nGot:\n{}", rust_code);
    }
}

// ============================================================================
// Phase 4: Multiple Classes
// ============================================================================

mod multiple_classes {
    use super::*;

    #[test]
    fn test_two_simple_independent_classes() {
        let python = r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

class Color:
    def __init__(self, r: int, g: int, b: int):
        self.r = r
        self.g = g
        self.b = b
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Point"),
            "Should have Point struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("struct Color"),
            "Should have Color struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("impl Point"),
            "Should have Point impl.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("impl Color"),
            "Should have Color impl.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_class_using_another_as_field_type() {
        let python = r#"
class Address:
    def __init__(self, street: str, city: str):
        self.street = street
        self.city = city

class Person:
    def __init__(self, name: str, address: Address):
        self.name = name
        self.address = address
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Address"),
            "Should have Address struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("struct Person"),
            "Should have Person struct.\nGot:\n{}",
            rust_code
        );

        let has_address_field = rust_code.contains("address") || rust_code.contains("Address");
        assert!(
            has_address_field,
            "Person should have address field.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_composition_class_contains_another() {
        let python = r#"
class Engine:
    def __init__(self, horsepower: int):
        self.horsepower = horsepower

    def start(self) -> str:
        return "Engine started"

class Car:
    def __init__(self, model: str):
        self.model = model
        self.engine = Engine(200)

    def start_car(self) -> str:
        return self.engine.start()
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Engine"),
            "Should have Engine struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("struct Car"),
            "Should have Car struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn start"),
            "Engine should have start method.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn start_car"),
            "Car should have start_car method.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_method_returning_another_class() {
        let python = r#"
class Position:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

class Robot:
    def __init__(self, name: str):
        self.name = name

    def get_position(self) -> Position:
        return Position(0, 0)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Position"),
            "Should have Position struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("struct Robot"),
            "Should have Robot struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn get_position"),
            "Robot should have get_position method.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_three_classes_interacting() {
        let python = r#"
class Author:
    def __init__(self, name: str):
        self.name = name

class Book:
    def __init__(self, title: str, author: Author):
        self.title = title
        self.author = author

class Library:
    def __init__(self, name: str):
        self.name = name

    def add_book(self, book: Book) -> str:
        return book.title
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Author"),
            "Should have Author struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("struct Book"),
            "Should have Book struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("struct Library"),
            "Should have Library struct.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_factory_pattern() {
        let python = r#"
class Widget:
    def __init__(self, id: int):
        self.id = id

class WidgetFactory:
    def __init__(self):
        self.counter = 0

    def create_widget(self) -> Widget:
        self.counter = self.counter + 1
        return Widget(self.counter)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Widget"),
            "Should have Widget struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("struct WidgetFactory"),
            "Should have WidgetFactory struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn create_widget"),
            "WidgetFactory should have create_widget method.\nGot:\n{}",
            rust_code
        );
    }
}

// ============================================================================
// Phase 5: @staticmethod Decorator
// ============================================================================

mod staticmethod {
    use super::*;

    #[test]
    fn test_simple_staticmethod_no_params() {
        let python = r#"
class Math:
    @staticmethod
    def pi() -> float:
        return 3.14159
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Math"),
            "Should have Math struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn pi"),
            "Should have pi function.\nGot:\n{}",
            rust_code
        );

        let has_self = rust_code.contains("&self") || rust_code.contains("& self");
        assert!(
            !has_self,
            "Staticmethod should not have &self parameter.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_staticmethod_with_parameters() {
        let python = r#"
class Calculator:
    @staticmethod
    def add(a: int, b: int) -> int:
        return a + b
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn add"),
            "Should have add function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("a") && rust_code.contains("b"),
            "Should have a and b parameters.\nGot:\n{}",
            rust_code
        );

        let has_self = rust_code.contains("&self") || rust_code.contains("& self");
        assert!(!has_self, "Staticmethod should not have &self.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_multiple_staticmethods() {
        let python = r#"
class Math:
    @staticmethod
    def add(a: int, b: int) -> int:
        return a + b

    @staticmethod
    def multiply(a: int, b: int) -> int:
        return a * b

    @staticmethod
    def subtract(a: int, b: int) -> int:
        return a - b
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn add"),
            "Should have add function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn multiply"),
            "Should have multiply function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn subtract"),
            "Should have subtract function.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_staticmethod_calling_another_staticmethod() {
        let python = r#"
class Math:
    @staticmethod
    def square(x: int) -> int:
        return x * x

    @staticmethod
    def sum_of_squares(a: int, b: int) -> int:
        return Math.square(a) + Math.square(b)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn square"),
            "Should have square function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn sum_of_squares"),
            "Should have sum_of_squares function.\nGot:\n{}",
            rust_code
        );

        let calls_square = rust_code.contains("square");
        assert!(calls_square, "Should call square method.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_mix_static_and_instance_methods() {
        let python = r#"
class Counter:
    @staticmethod
    def default_start() -> int:
        return 0

    def __init__(self):
        self.count = Counter.default_start()

    def increment(self) -> None:
        self.count = self.count + 1

    def get_count(self) -> int:
        return self.count

    @staticmethod
    def max_count() -> int:
        return 100
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn default_start"),
            "Should have default_start function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn max_count"),
            "Should have max_count function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn increment"),
            "Should have increment method.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn get_count"),
            "Should have get_count method.\nGot:\n{}",
            rust_code
        );
    }
}

// ============================================================================
// Phase 6: @classmethod Decorator
// ============================================================================

mod classmethod {
    use super::*;

    #[test]
    fn test_simple_classmethod_factory() {
        let python = r#"
class Person:
    def __init__(self, name: str):
        self.name = name

    @classmethod
    def create_john(cls):
        return cls("John")
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("struct Person"),
            "Should have Person struct.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn create_john"),
            "Should have create_john function.\nGot:\n{}",
            rust_code
        );

        let has_self_param = rust_code.contains("&self") || rust_code.contains("cls:");
        assert!(
            !has_self_param,
            "Classmethod should not have &self or cls parameter.\nGot:\n{}",
            rust_code
        );

        let has_constructor =
            rust_code.contains("Self::new") || rust_code.contains("Person::new") || rust_code.contains("Self {");
        assert!(
            has_constructor,
            "Should use Self::new or constructor.\nGot:\n{}",
            rust_code
        );

        let has_cls_var = rust_code.contains("cls(");
        assert!(
            !has_cls_var,
            "Should not have undefined cls variable.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_classmethod_with_parameters() {
        let python = r#"
class User:
    def __init__(self, name: str, age: int):
        self.name = name
        self.age = age

    @classmethod
    def create(cls, name: str, age: int):
        return cls(name, age)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn create"),
            "Should have create function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("name") && rust_code.contains("age"),
            "Should have name and age parameters.\nGot:\n{}",
            rust_code
        );

        let has_cls_param = rust_code.contains("cls:");
        assert!(!has_cls_param, "Should not have cls parameter.\nGot:\n{}", rust_code);

        let has_constructor_call = rust_code.contains("Self::new") || rust_code.contains("User::new");
        assert!(has_constructor_call, "Should call constructor.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_multiple_classmethods() {
        let python = r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

    @classmethod
    def origin(cls):
        return cls(0, 0)

    @classmethod
    def unit_x(cls):
        return cls(1, 0)

    @classmethod
    def unit_y(cls):
        return cls(0, 1)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn origin"),
            "Should have origin function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn unit_x"),
            "Should have unit_x function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn unit_y"),
            "Should have unit_y function.\nGot:\n{}",
            rust_code
        );

        let cls_count = rust_code.matches("cls:").count();
        assert_eq!(
            cls_count, 0,
            "No function should have cls parameter.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_classmethod_calling_another_classmethod() {
        let python = r#"
class Rectangle:
    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height

    @classmethod
    def square(cls, size: int):
        return cls(size, size)

    @classmethod
    def unit_square(cls):
        return cls.square(1)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn square"),
            "Should have square function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn unit_square"),
            "Should have unit_square function.\nGot:\n{}",
            rust_code
        );

        let calls_square = rust_code.contains("Self::square") || rust_code.contains("Rectangle::square");
        assert!(calls_square, "Should call square classmethod.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_classmethod_as_alternative_constructor() {
        let python = r#"
class Date:
    def __init__(self, year: int, month: int, day: int):
        self.year = year
        self.month = month
        self.day = day

    @classmethod
    def from_string(cls, date_str: str):
        return cls(2024, 1, 1)

    @classmethod
    def today(cls):
        return cls(2024, 10, 9)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn from_string"),
            "Should have from_string function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn today"),
            "Should have today function.\nGot:\n{}",
            rust_code
        );

        let has_constructors =
            rust_code.contains("Self::new") || rust_code.contains("Date::new") || rust_code.contains("Self {");
        assert!(has_constructors, "Should have constructor calls.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_mix_class_static_instance_methods() {
        let python = r#"
class Calculator:
    DEFAULT_OFFSET: int = 10

    def __init__(self, offset: int):
        self.offset = offset

    @staticmethod
    def add(a: int, b: int) -> int:
        return a + b

    @classmethod
    def with_default_offset(cls):
        return cls(cls.DEFAULT_OFFSET)

    def add_with_offset(self, a: int, b: int) -> int:
        return Calculator.add(a, b) + self.offset
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();
        assert!(
            rust_code.contains("fn add"),
            "Should have add static function.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn with_default_offset"),
            "Should have with_default_offset classmethod.\nGot:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains("fn add_with_offset"),
            "Should have add_with_offset instance method.\nGot:\n{}",
            rust_code
        );

        let has_self = rust_code.contains("&self") || rust_code.contains("& self");
        assert!(has_self, "add_with_offset should have &self.\nGot:\n{}", rust_code);
    }
}
