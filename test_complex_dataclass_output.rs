#[doc = "// NOTE: Map Python module 'dataclasses'()"]
use std::collections::HashMap;
use std::collections::HashSet;
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ComplexState {
    pub events: HashSet<String>,
    pub items: Vec<i32>,
    pub metadata: HashMap<String, i32>,
    pub count: i32,
    pub name: String,
    pub enabled: bool,
    pub id: i32,
}
impl ComplexState {
    pub fn new(id: i32) -> Self {
        Self {
            events: HashSet::new(),
            items: Vec::new(),
            metadata: HashMap::new(),
            count: 0,
            name: "unknown".to_string(),
            enabled: Default::default(),
            id,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "events" => Some(Box::new(self.events.clone()) as Box<dyn std::any::Any>),
            "items" => Some(Box::new(self.items.clone()) as Box<dyn std::any::Any>),
            "metadata" => Some(Box::new(self.metadata.clone()) as Box<dyn std::any::Any>),
            "count" => Some(Box::new(self.count.clone()) as Box<dyn std::any::Any>),
            "name" => Some(Box::new(self.name.clone()) as Box<dyn std::any::Any>),
            "enabled" => Some(Box::new(self.enabled.clone()) as Box<dyn std::any::Any>),
            "id" => Some(Box::new(self.id.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "events" => {
                if let Some(v) = value.downcast_ref::<HashSet<String>>() {
                    self.events = v.clone();
                    true
                } else {
                    false
                }
            }
            "items" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.items = v.clone();
                    true
                } else {
                    false
                }
            }
            "metadata" => {
                if let Some(v) = value.downcast_ref::<HashMap<String, i32>>() {
                    self.metadata = v.clone();
                    true
                } else {
                    false
                }
            }
            "count" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.count = v.clone();
                    true
                } else {
                    false
                }
            }
            "name" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.name = v.clone();
                    true
                } else {
                    false
                }
            }
            "enabled" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.enabled = v.clone();
                    true
                } else {
                    false
                }
            }
            "id" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.id = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
