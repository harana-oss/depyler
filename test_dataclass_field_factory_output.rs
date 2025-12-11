#[doc = "// NOTE: Map Python module 'dataclasses'()"]
use std::collections::HashSet;
#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    pub executed_events: HashSet<String>,
}
impl State {
    pub fn new() -> Self {
        Self {
            executed_events: HashSet::new(),
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "executed_events" => {
                Some(Box::new(self.executed_events.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "executed_events" => {
                if let Some(v) = value.downcast_ref::<HashSet<String>>() {
                    self.executed_events = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
