#[derive(Debug, Clone)]
pub struct TestClass {
    pub seconds: i32,
}
impl TestClass {
    pub fn new() -> Self {
        Self { seconds: 0 }
    }
}
#[doc = "Test simple attribute access: test.seconds"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_attribute_access(test: &TestClass) -> i32 {
    (test.num_seconds() % 86400) as i32
}
 