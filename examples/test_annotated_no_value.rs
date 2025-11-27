pub const counter: i32 = 10;
pub const name: &str = "test";
pub const is_valid: bool = true;
pub const score: f64 = 3.14;
pub const items: serde_json::Value = vec![1, 2, 3];
pub const mapping: serde_json::Value = {
    let mut map = HashMap::new();
    map.insert("a".to_string(), 1);
    map.insert("b".to_string(), 2);
    map
};
pub const unique_values: serde_json::Value = {
    let mut set = HashSet::new();
    set.insert("x");
    set.insert("y");
    set.insert("z");
    set
};
pub const field_position: serde_json::Value = FieldPosition::new(5, 10);
pub const new_team: &str = "TeamA";
pub const valid: bool = true;
pub const maybe_value: &str = "found";
use serde_json;
use std::collections::HashMap;
use std::collections::HashSet;
#[derive(Debug, Clone)]
pub struct FieldPosition {
    pub x: i32,
    pub y: i32,
}
impl FieldPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
