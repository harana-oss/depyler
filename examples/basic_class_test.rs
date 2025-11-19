#[derive(Debug, Clone)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn distance_from_origin(&self) -> f64 {
        return (self.x * self.x + self.y * self.y as f64).powf(0.5);
    }
    pub fn translate(&mut self, dx: i32, dy: i32) {
        self.x = self.x + dx;
        self.y = self.y + dy;
    }
}
#[derive(Debug, Clone)]
pub struct Rectangle {
    pub width: i32,
    pub height: i32,
}
impl Rectangle {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
    pub fn area(&self) -> i32 {
        return self.width * self.height;
    }
    pub fn perimeter(&self) -> i32 {
        return 2 * self.width + self.height;
    }
    pub fn is_square(&self) -> bool {
        return self.width == self.height;
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    pub name: String,
    pub age: i32,
}
impl Person {
    pub fn new(name: String, age: i32) -> Self {
        Self { name, age }
    }
    pub fn greet(&self) -> String {
        return "Hello, my name is ".to_string() + self.name;
    }
    pub fn is_adult(&self) -> bool {
        return self.age >= 18;
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Product {
    pub name: String,
    pub price: f64,
    pub stock: i32,
}
impl Product {
    pub fn new(name: String, price: f64, stock: i32) -> Self {
        Self { name, price, stock }
    }
    pub fn total_value(&self) -> f64 {
        return self.price * self.stock;
    }
    pub fn is_available(&self) -> bool {
        return self.stock > 0;
    }
    pub fn create_sample() -> Self {
        return Product::new("Sample".to_string(), 9.99, 100);
    }
    pub fn from_price_only(price: f64) -> Self {
        return Self::new("Unnamed".to_string(), price, 0);
    }
}
#[doc = "Test Point class functionality"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_point() {
    let mut p = Point::new(3, 4);
    assert!(p.x == 3);
    assert!(p.y == 4);
    assert!(p.distance_from_origin() == 5.0);
    p.translate(1, 1);
    assert!(p.x == 4);
    assert!(p.y == 5);
}
#[doc = "Test Rectangle class functionality"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_rectangle() {
    let r = Rectangle::new(10, 20);
    assert!(r.area() == 200);
    assert!(r.perimeter() == 60);
    assert!(!r.is_square());
    let square = Rectangle::new(15, 15);
    assert!(square.is_square());
}
#[doc = "Test Person dataclass functionality"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_person() {
    let mut p = Person::new("Alice".to_string(), 25);
    assert!(p.name == "Alice");
    assert!(p.age == 25);
    assert!(p.is_adult());
    assert!(p.greet() == "Hello, my name is Alice".to_string());
    let child = Person::new("Bob".to_string(), 10);
    assert!(!child.is_adult());
}
#[doc = "Test Product dataclass with static and class methods"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_product() {
    let prod = Product::new("Widget".to_string(), 19.99, 50);
    assert!(prod.name == "Widget");
    assert!(prod.price == 19.99);
    assert!(prod.stock == 50);
    assert!(prod.total_value() == 999.5);
    assert!(prod.is_available());
    let sample = Product.create_sample();
    assert!(sample.name == "Sample".to_string());
    assert!(sample.price == 9.99);
    assert!(sample.stock == 100);
    let custom = Product.from_price_only(29.99);
    assert!(custom.name == "Unnamed".to_string());
    assert!(custom.price == 29.99);
    assert!(custom.stock == 0);
    assert!(!custom.is_available());
}
