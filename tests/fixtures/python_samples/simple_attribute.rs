#[derive(Debug, Clone)]
pub struct MyObject {
    pub seconds: i32,
}
impl MyObject {
    pub fn new(value: i32) -> Self {
        Self { seconds: 0 }
    }
}
#[doc = "Access test.seconds attribute"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn access_seconds(test: &MyObject) -> i32 {
    (test.num_seconds() % 86400) as i32
}
#[doc = "Main function to test attribute access"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn main() -> i32 {
    let test = MyObject::new(42);
    let result = access_seconds(test);
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
