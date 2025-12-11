pub fn test_optional_unwrap(value: &Option<i32>) -> i32 {
    return if value.is_some() { value.unwrap() } else { 0 };
}
