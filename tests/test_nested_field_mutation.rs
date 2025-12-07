#[derive(Debug, Clone)]
pub struct Inner {}
impl Inner {
    pub fn new() -> Self {
        Self {}
    }
}
#[derive(Debug, Clone)]
pub struct Outer {}
impl Outer {
    pub fn new() -> Self {
        Self {}
    }
}
#[derive(Debug, Clone)]
pub struct State {}
impl State {
    pub fn new() -> Self {
        Self {}
    }
}
#[doc = "Test that nested field mutation properly marks the variable as mutable."]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn mutate_nested_field(state: &State, value: i32) {
    let mut obj = state.data;
    obj.inner.values.push(value);
}
