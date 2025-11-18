#[derive(Debug, Clone)]
pub struct ZeroDivisionError {
    message: String,
}
impl std::fmt::Display for ZeroDivisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "division by zero: {}", self.message)
    }
}
impl std::error::Error for ZeroDivisionError {}
impl ZeroDivisionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
#[doc = "Test round() with int first argument and int second argument"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_round_int_with_precision() {
    let _cse_temp_0 = (123 as f64).round() as i32;
    let result1 = _cse_temp_0;
    let _cse_temp_1 = (123 as f64).round() as i32;
    let result2 = _cse_temp_1;
    (result1, result2, result3, result4)
}
#[doc = "Test round() with float first argument and int second argument"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_round_float_with_precision() {
    let _cse_temp_0 = (3.14159 as f64).round() as i32;
    let result1 = _cse_temp_0;
    let _cse_temp_1 = (1.5 as f64).round() as i32;
    let result3 = _cse_temp_1;
    (result1, result2, result3, result4, result5)
}
#[doc = "Test round() with negative precision"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn round_with_negative_precision(x: f64, precision: i32) -> f64 {
    (x as f64).round() as i32
}
#[doc = "Round integer to specified decimal places"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn round_int_to_decimal(value: i32, places: i32) -> i32 {
    (value as f64).round() as i32
}
#[doc = "Round float to specified decimal places"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn round_float_to_decimal(value: f64, places: i32) -> f64 {
    (value as f64).round() as i32
}
#[doc = "Test round() with variable as first argument"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn round_variable_with_precision() {
    let x = 3.14159;
    let _cse_temp_0 = (x as f64).round() as i32;
    let result1 = _cse_temp_0;
    let y = 123.456;
    let _cse_temp_1 = (y as f64).round() as i32;
    let result2 = _cse_temp_1;
    let z = 999.999;
    let _cse_temp_2 = (z as f64).round() as i32;
    let result3 = _cse_temp_2;
    let a = 999.999;
    let _cse_temp_3 = ((a.num_seconds() % 86400) as i32 as f64).round() as i32;
    let resulta = _cse_temp_3;
    (result1, result2, result3, resulta)
}
#[doc = "Test round() with integer variable as first argument"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn round_variable_int_with_precision() {
    let value = 127;
    let _cse_temp_0 = (value as f64).round() as i32;
    let result1 = _cse_temp_0;
    let large_num = 12345;
    let _cse_temp_1 = (large_num as f64).round() as i32;
    let result2 = _cse_temp_1;
    (result1, result2)
}
#[doc = "Test round() with computed value as first argument"]
#[doc = " Depyler: proven to terminate"]
pub fn round_computed_value() -> Result<(), ZeroDivisionError> {
    let a = 10.5;
    let b = 3.2;
    let _cse_temp_0 = (a + b as f64).round() as i32;
    let result1 = _cse_temp_0;
    let c = 100.0;
    let d = 3.0;
    let _cse_temp_1 = c / d;
    let _cse_temp_2 = (_cse_temp_1 as f64).round() as i32;
    let result2 = _cse_temp_2;
    Ok((result1, result2))
}
#[doc = "Test min () with int and float arguments"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_min_int_float() {
    let _cse_temp_0 = std::cmp::min(5, 3.14);
    let result1 = _cse_temp_0;
    let _cse_temp_1 = std::cmp::min(10, 10.5);
    let result2 = _cse_temp_1;
    let _cse_temp_2 = std::cmp::min(-3, -2.5);
    let result3 = _cse_temp_2;
    let _cse_temp_3 = std::cmp::min(0, 0.1);
    let result4 = _cse_temp_3;
    (result1, result2, result3, result4)
}
#[doc = "Test min () with variable int and float arguments"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_min_variable_int_float() {
    let x = 5;
    let _cse_temp_0 = std::cmp::min(x, 3.14);
    let result1 = _cse_temp_0;
    let y = 10;
    let _cse_temp_1 = std::cmp::min(y, 10.5);
    let result2 = _cse_temp_1;
    let z = -3;
    let _cse_temp_2 = std::cmp::min(z, -2.5);
    let result3 = _cse_temp_2;
    let w = 0;
    let _cse_temp_3 = std::cmp::min(w, 0.1);
    let result4 = _cse_temp_3;
    (result1, result2, result3, result4)
}
#[doc = "Test min () with float and float arguments"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn test_min_float_float() {
    let _cse_temp_0 = std::cmp::min(3.14, 2.71);
    let result1 = _cse_temp_0;
    let _cse_temp_1 = std::cmp::min(10.5, 10.5);
    let result2 = _cse_temp_1;
    let _cse_temp_2 = std::cmp::min(-2.5, -3.7);
    let result3 = _cse_temp_2;
    let _cse_temp_3 = std::cmp::min(0.0, 0.1);
    let result4 = _cse_temp_3;
    (result1, result2, result3, result4)
}
#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, TestResult};
    #[test]
    fn test_round_int_to_decimal_examples() {
        assert_eq!(round_int_to_decimal(0, 0), 0);
        assert_eq!(round_int_to_decimal(1, 2), 3);
        assert_eq!(round_int_to_decimal(-1, 1), 0);
    }
}
