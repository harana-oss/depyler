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

use crate::test_helpers::transpile;

// ============================================================================
// Phase 1: Simple Classes with __init__
// ============================================================================

mod basic_classes {
    use super::*;

    #[test]
    fn test_class_struct_generation() {
        // Simple class with init
        let python = r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("struct Point"), "Should generate struct Point: {}", rust_code);
        assert!(rust_code.contains("x:") && rust_code.contains("y:"), "Should have x and y fields: {}", rust_code);
        assert!(rust_code.contains("impl Point"), "Should generate impl block: {}", rust_code);
        assert!(rust_code.contains("fn new"), "Should have new() constructor: {}", rust_code);

        // Class with typed fields
        let python = r#"
class Rectangle:
    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("struct Rectangle"), "Should generate struct Rectangle: {}", rust_code);
        assert!(rust_code.contains("width") && rust_code.contains("height"), "Should have width/height fields: {}", rust_code);

        // Class with default parameters
        let python = r#"
class Counter:
    def __init__(self, start: int = 0):
        self.value = start
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("struct Counter"), "Should generate struct Counter: {}", rust_code);
        assert!(rust_code.contains("value"), "Should have value field: {}", rust_code);

        // Empty class
        let python = r#"
class Empty:
    pass
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("struct Empty"), "Should generate struct Empty: {}", rust_code);
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
        let rust_code = transpile(python);
        assert!(rust_code.contains("struct Student"), "Should generate struct Student: {}", rust_code);
        assert!(
            rust_code.contains("name") && rust_code.contains("age") && rust_code.contains("grade") && rust_code.contains("active"),
            "Should have all 4 fields: {}", rust_code
        );
    }

    #[test]
    fn test_class_usage_patterns() {
        // Class instantiation
        let python = r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

def create_point() -> Point:
    p = Point(10, 20)
    return p
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("Point::new") || rust_code.contains("Point {"), "Should create Point instance: {}", rust_code);

        // Field access
        let python = r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

def get_x(p: Point) -> int:
    return p.x
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("p.x") || rust_code.contains("p .x"), "Should access field p.x: {}", rust_code);

        // Field mutation
        let python = r#"
class Counter:
    def __init__(self, value: int):
        self.value = value

def increment(c: Counter) -> int:
    c.value = c.value + 1
    return c.value
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("c.value") || rust_code.contains("c .value"), "Should access and mutate c.value: {}", rust_code);

        // Computed field
        let python = r#"
class Rectangle:
    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height
        self.area = width * height
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("struct Rectangle"), "Should generate struct Rectangle: {}", rust_code);
        assert!(rust_code.contains("area"), "Should have computed area field: {}", rust_code);
    }
}

// ============================================================================
// Phase 2: Instance Methods
// ============================================================================

mod instance_methods {
    use super::*;

    #[test]
    fn test_basic_instance_methods() {
        // Simple instance method
        let python = r#"
class Counter:
    def __init__(self, value: int):
        self.value = value

    def get_value(self) -> int:
        return self.value
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("impl Counter"), "Should have impl Counter block: {}", rust_code);
        assert!(rust_code.contains("fn get_value"), "Should have get_value method: {}", rust_code);
        let has_self_ref = rust_code.contains("&self") || rust_code.contains("& self");
        assert!(has_self_ref, "Method should take &self parameter: {}", rust_code);

        // Method accessing fields
        let python = r#"
class Rectangle:
    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height

    def area(self) -> int:
        return self.width * self.height
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("fn area"), "Should have area method: {}", rust_code);
        let has_width = rust_code.contains("self.width") || rust_code.contains("self .width");
        let has_height = rust_code.contains("self.height") || rust_code.contains("self .height");
        assert!(has_width && has_height, "Should access self.width and self.height: {}", rust_code);

        // Method modifying self
        let python = r#"
class Counter:
    def __init__(self, value: int):
        self.value = value

    def increment(self) -> None:
        self.value = self.value + 1
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("fn increment"), "Should have increment method: {}", rust_code);
        let has_mut_self = rust_code.contains("&mut self") || rust_code.contains("& mut self");
        assert!(has_mut_self, "Mutating method should take &mut self: {}", rust_code);
    }

    #[test]
    fn test_methods_with_parameters() {
        // Method with parameters
        let python = r#"
class Calculator:
    def __init__(self, initial: int):
        self.value = initial

    def add(self, amount: int) -> int:
        return self.value + amount
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("fn add"), "Should have add method: {}", rust_code);
        assert!(rust_code.contains("amount"), "Should have amount parameter: {}", rust_code);

        // Method calling another method
        let python = r#"
class Circle:
    def __init__(self, radius: float):
        self.radius = radius

    def diameter(self) -> float:
        return self.radius * 2.0

    def circumference(self) -> float:
        return self.diameter() * 3.14159
"#;
        let rust_code = transpile(python);
        assert!(rust_code.contains("fn diameter"), "Should have diameter method: {}", rust_code);
        assert!(rust_code.contains("fn circumference"), "Should have circumference method: {}", rust_code);
        let calls_diameter = rust_code.contains("self.diameter()") || rust_code.contains("self .diameter");
        assert!(calls_diameter, "Should call self.diameter(): {}", rust_code);
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
        let rust_code = transpile(python);
        assert!(rust_code.contains("fn deposit"), "Should have deposit method: {}", rust_code);
        assert!(rust_code.contains("fn withdraw"), "Should have withdraw method: {}", rust_code);
        assert!(rust_code.contains("fn get_balance"), "Should have get_balance method: {}", rust_code);
    }

    #[test]
    fn test_simple_method_call() {
        let python_code = r#"
def simple_append():
    numbers = [1, 2, 3]
    numbers.append(4)
    return numbers
"#;
        let rust_code = transpile(python_code);
        assert!(rust_code.contains("push"), "Should use push for append: {}", rust_code);
    }
}

// ============================================================================
// Phase 3: Class-Level Attributes
// ============================================================================

mod class_attributes {
    use super::*;

    #[test]
    fn test_class_constants_and_attributes() {
        // Tests: simple class constant, class attribute access via classname,
        // multiple class attributes, class attributes used in methods,
        // mix of class and instance attributes
        let test_cases = [
            (
                "simple class constant",
                r#"
class Config:
    MAX_SIZE: int = 100

    def __init__(self, size: int):
        self.size = size
"#,
                vec!["struct Config"],
                vec!["MAX_SIZE", "max_size", "MAX_SIZE: i32 = 100"],
            ),
            (
                "class attribute access via classname",
                r#"
class Math:
    PI: float = 3.14159

    def get_pi(self) -> float:
        return Math.PI
"#,
                vec!["struct Math", "fn get_pi"],
                vec!["PI", "pi"],
            ),
            (
                "multiple class attributes",
                r#"
class Constants:
    WIDTH: int = 800
    HEIGHT: int = 600
    TITLE: str = "Game"

    def __init__(self):
        self.width = Constants.WIDTH
        self.height = Constants.HEIGHT
"#,
                vec![],
                vec!["WIDTH", "width", "HEIGHT", "height", "TITLE", "title"],
            ),
            (
                "class attribute used in method",
                r#"
class Circle:
    PI: float = 3.14159

    def __init__(self, radius: float):
        self.radius = radius

    def area(self) -> float:
        return Circle.PI * self.radius * self.radius
"#,
                vec!["struct Circle", "fn area"],
                vec!["PI", "pi"],
            ),
            (
                "mix class and instance attributes",
                r#"
class Car:
    WHEELS: int = 4

    def __init__(self, color: str):
        self.color = color
        self.wheels = Car.WHEELS

    def get_wheels(self) -> int:
        return self.wheels
"#,
                vec!["struct Car", "color"],
                vec!["WHEELS", "wheels"],
            ),
        ];

        for (name, python, required, any_of) in test_cases {
            let rust_code = transpile(python);

            for req in &required {
                assert!(
                    rust_code.contains(req),
                    "{}: Should contain '{}'\nGot:\n{}",
                    name, req, rust_code
                );
            }

            if !any_of.is_empty() {
                let has_any = any_of.iter().any(|s| rust_code.contains(s));
                assert!(
                    has_any,
                    "{}: Should contain one of {:?}\nGot:\n{}",
                    name, any_of, rust_code
                );
            }
        }
    }
}

// ============================================================================
// Phase 4: Multiple Classes
// ============================================================================

mod multiple_classes {
    use super::*;

    #[test]
    fn test_multiple_class_definitions() {
        // Tests: two independent classes, class using another as field type,
        // composition pattern, method returning another class,
        // three classes interacting, factory pattern
        let test_cases = [
            (
                "two independent classes",
                r#"
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

class Color:
    def __init__(self, r: int, g: int, b: int):
        self.r = r
        self.g = g
        self.b = b
"#,
                vec!["struct Point", "struct Color", "impl Point", "impl Color"],
            ),
            (
                "class using another as field type",
                r#"
class Address:
    def __init__(self, street: str, city: str):
        self.street = street
        self.city = city

class Person:
    def __init__(self, name: str, address: Address):
        self.name = name
        self.address = address
"#,
                vec!["struct Address", "struct Person", "address"],
            ),
            (
                "composition pattern",
                r#"
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
"#,
                vec!["struct Engine", "struct Car", "fn start", "fn start_car"],
            ),
            (
                "method returning another class",
                r#"
class Position:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

class Robot:
    def __init__(self, name: str):
        self.name = name

    def get_position(self) -> Position:
        return Position(0, 0)
"#,
                vec!["struct Position", "struct Robot", "fn get_position"],
            ),
            (
                "three classes interacting",
                r#"
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
"#,
                vec!["struct Author", "struct Book", "struct Library"],
            ),
            (
                "factory pattern",
                r#"
class Widget:
    def __init__(self, id: int):
        self.id = id

class WidgetFactory:
    def __init__(self):
        self.counter = 0

    def create_widget(self) -> Widget:
        self.counter = self.counter + 1
        return Widget(self.counter)
"#,
                vec!["struct Widget", "struct WidgetFactory", "fn create_widget"],
            ),
        ];

        for (name, python, required) in test_cases {
            let rust_code = transpile(python);

            for req in &required {
                assert!(
                    rust_code.contains(req),
                    "{}: Should contain '{}'\nGot:\n{}",
                    name, req, rust_code
                );
            }
        }
    }
}

// ============================================================================
// Phase 5: @staticmethod Decorator
// ============================================================================

mod staticmethod {
    use super::*;

    #[test]
    fn test_staticmethod_basics() {
        // Tests: staticmethod without params, with params, multiple staticmethods
        let test_cases = [
            (
                "staticmethod no params",
                r#"
class Math:
    @staticmethod
    def pi() -> float:
        return 3.14159
"#,
                vec!["struct Math", "fn pi"],
            ),
            (
                "staticmethod with parameters",
                r#"
class Calculator:
    @staticmethod
    def add(a: int, b: int) -> int:
        return a + b
"#,
                vec!["fn add"],
            ),
            (
                "multiple staticmethods",
                r#"
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
"#,
                vec!["fn add", "fn multiply", "fn subtract"],
            ),
            (
                "staticmethod calling another staticmethod",
                r#"
class Math:
    @staticmethod
    def square(x: int) -> int:
        return x * x

    @staticmethod
    def sum_of_squares(a: int, b: int) -> int:
        return Math.square(a) + Math.square(b)
"#,
                vec!["fn square", "fn sum_of_squares"],
            ),
            (
                "mix static and instance methods",
                r#"
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
"#,
                vec!["fn default_start", "fn max_count", "fn increment", "fn get_count"],
            ),
        ];

        for (name, python, required) in test_cases {
            let rust_code = transpile(python);

            for req in &required {
                assert!(
                    rust_code.contains(req),
                    "{}: Should contain '{}'\nGot:\n{}",
                    name, req, rust_code
                );
            }
        }
    }
}

// ============================================================================
// Phase 6: @classmethod Decorator
// ============================================================================

mod classmethod {
    use super::*;

    #[test]
    fn test_classmethod_basics() {
        // Tests classmethod transpilation generates correct function signatures
        let test_cases = [
            (
                "simple classmethod factory",
                r#"
class Person:
    def __init__(self, name: str):
        self.name = name

    @classmethod
    def create_john(cls):
        return cls("John")
"#,
                vec!["struct Person", "fn create_john"],
                vec!["Self::new", "Person::new", "Self {"],  // any_of
            ),
            (
                "classmethod with parameters",
                r#"
class User:
    def __init__(self, name: str, age: int):
        self.name = name
        self.age = age

    @classmethod
    def create(cls, name: str, age: int):
        return cls(name, age)
"#,
                vec!["fn create", "name", "age"],
                vec!["Self::new", "User::new"],
            ),
            (
                "multiple classmethods",
                r#"
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
"#,
                vec!["fn origin", "fn unit_x", "fn unit_y"],
                vec![],
            ),
            (
                "classmethod calling another classmethod",
                r#"
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
"#,
                vec!["fn square", "fn unit_square"],
                vec!["Self::square", "Rectangle::square"],
            ),
            (
                "classmethod as alternative constructor",
                r#"
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
"#,
                vec!["fn from_string", "fn today"],
                vec!["Self::new", "Date::new", "Self {"],
            ),
            (
                "mix static, class, and instance methods",
                r#"
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
"#,
                vec!["fn add", "fn with_default_offset", "fn add_with_offset"],
                vec![],
            ),
        ];

        for (name, python, required, any_of) in test_cases {
            let rust_code = transpile(python);

            for req in &required {
                assert!(
                    rust_code.contains(req),
                    "{}: Should contain '{}'\nGot:\n{}",
                    name, req, rust_code
                );
            }

            if !any_of.is_empty() {
                let has_any = any_of.iter().any(|s| rust_code.contains(s));
                assert!(
                    has_any,
                    "{}: Should contain one of {:?}\nGot:\n{}",
                    name, any_of, rust_code
                );
            }

            // Classmethod should not have cls: parameter in the Rust output
            assert!(
                !rust_code.contains("cls:"),
                "{}: Should not have cls: parameter\nGot:\n{}",
                name, rust_code
            );
        }
    }
}
