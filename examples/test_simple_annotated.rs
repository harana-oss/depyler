#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_annotated_assignments() {
    let mut counter: i32 = 0;
    let mut name: String = "".to_string();
    let mut is_valid: bool = false;
    let mut score: f64 = 0.0;
    counter = 10;
    name = "test".to_string();
    is_valid = true;
    score = 3.14;
    println!("{}", counter);
    println!("{}", name);
    println!("{}", is_valid);
    println!("{}", score);
}
