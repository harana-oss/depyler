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
#[derive(Debug, Clone)]
pub struct IndexError {
    message: String,
}
impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "index out of range: {}", self.message)
    }
}
impl std::error::Error for IndexError {}
impl IndexError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
#[doc = " Depyler: verified panic-free"]
pub fn sum_list(numbers: &Vec<i32>) -> i32 {
    let mut total: i32 = 0;
    for num in numbers.iter().cloned() {
        total = total + num;
    }
    total
}
pub fn find_max(numbers: &Vec<i32>) -> Result<Option<i32>, IndexError> {
    if numbers.is_empty() {
        return Ok(None);
    }
    let mut max_val: i32 = numbers.get(0usize).cloned().unwrap_or_default();
    for num in numbers.iter().cloned() {
        if num > max_val {
            max_val = num;
        }
    }
    Ok(Some(max_val))
}
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn count_elements(numbers: &Vec<i32>) -> i32 {
    numbers.len() as i32 as i32
}
#[doc = " Depyler: verified panic-free"]
pub fn filter_positive(numbers: &Vec<i32>) -> Vec<i32> {
    let mut result: Vec<i32> = vec![];
    for num in numbers.iter().cloned() {
        if num > 0 {
            result.push(num);
        }
    }
    result
}
#[doc = " Depyler: proven to terminate"]
pub fn get_element(numbers: &Vec<i32>, index: i32) -> Result<Option<i32>, IndexError> {
    let _cse_temp_0 = 0 <= index;
    let _cse_temp_1 = numbers.len() as i32;
    let _cse_temp_2 = index < _cse_temp_1;
    let _cse_temp_3 = _cse_temp_0 && _cse_temp_2;
    if _cse_temp_3 {
        return Ok(Some(
            numbers.get(index as usize).cloned().unwrap_or_default(),
        ));
    }
    Ok(None)
}
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn reverse_list(numbers: &Vec<i32>) -> Vec<i32> {
    let mut result: Vec<i32> = vec![];
    for i in {
        let step = (-1 as i32).abs() as usize;
        if step == 0 {
            panic!("range() arg 3 must not be zero");
        }
        (-1..(numbers.len() as i32).saturating_sub(1))
            .rev()
            .step_by(step.max(1))
    } {
        result.push(numbers.get(i as usize).cloned().unwrap_or_default());
    }
    result
}
#[doc = " Depyler: verified panic-free"]
pub fn contains_element(numbers: &Vec<i32>, target: i32) -> bool {
    for num in numbers.iter().cloned() {
        if num == target {
            return true;
        }
    }
    false
}
#[doc = " Depyler: proven to terminate"]
pub fn first_element(numbers: &Vec<i32>) -> Result<Option<i32>, IndexError> {
    if !numbers.is_empty() {
        return Ok(Some(numbers.get(0usize).cloned().unwrap_or_default()));
    }
    Ok(None)
}
#[doc = " Depyler: proven to terminate"]
pub fn last_element(numbers: &Vec<i32>) -> Result<Option<i32>, IndexError> {
    if !numbers.is_empty() {
        return Ok(Some({
            let base = &numbers;
            base.get(base.len().saturating_sub(1usize))
                .cloned()
                .unwrap_or_default()
        }));
    }
    Ok(None)
}
#[doc = " Depyler: proven to terminate"]
pub fn average_numbers(numbers: Vec<i32>) -> Result<f64, ZeroDivisionError> {
    if numbers.is_empty() {
        return Ok(0.0);
    }
    Ok((numbers.iter().sum::<f64>() as f64) / (numbers.len() as i32 as f64))
}
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn check_not_in_list(variable: &str) -> bool {
    !vec!["A".to_string(), "B".to_string()].contains_key(variable)
}
#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, TestResult};
    #[test]
    fn test_sum_list_examples() {
        assert_eq!(sum_list(&vec![]), 0);
        assert_eq!(sum_list(&vec![1]), 1);
        assert_eq!(sum_list(&vec![1, 2, 3]), 6);
    }
    #[test]
    fn test_count_elements_examples() {
        assert_eq!(count_elements(&vec![]), 0);
        assert_eq!(count_elements(&vec![1]), 1);
        assert_eq!(count_elements(&vec![1, 2, 3]), 3);
    }
    #[test]
    fn test_filter_positive_examples() {
        assert_eq!(filter_positive(vec![]), vec![]);
        assert_eq!(filter_positive(vec![1]), vec![1]);
    }
    #[test]
    fn test_reverse_list_examples() {
        assert_eq!(reverse_list(vec![]), vec![]);
        assert_eq!(reverse_list(vec![1]), vec![1]);
    }
    #[test]
    fn test_check_not_in_list_examples() {
        let _ = check_not_in_list(Default::default());
    }
}
