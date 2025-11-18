#[derive(Debug, Clone)]
pub struct TestObject {
    pub seconds: i32,
}
impl TestObject {
    pub fn new(value: i32) -> Self {
        Self { seconds: 0 }
    }
}
#[doc = "\n    BUG REPRODUCTION: Access test.seconds\n    \n    Expected Rust output: test.seconds\n    Actual Rust output:(test.num_seconds() % 86400) as i32\n    \n    This is a bug where the transpiler treats 'seconds' as a special\n    attribute name related to time/datetime objects, even though it's\n    just a regular struct field.\n    "]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_seconds_attribute_access(test: &TestObject) -> i32 {
    (test.num_seconds() % 86400) as i32
}
#[doc = "Test the bug"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn main() -> i32 {
    let test = TestObject::new(42);
    let result = test_seconds_attribute_access(test);
    assert!(result == 42, "{}", format!("Expected 42, got {}", result));
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, TestResult};
    #[test]
    fn test_main_examples() {
        let _ = main();
    }
}
