#[derive(Debug, Clone)]
pub struct DataObject {
    pub x: i32,
    pub y: i32,
    pub seconds: i32,
}
impl DataObject {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y, seconds: 0 }
    }
}
#[doc = "Access obj.x"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn get_x_attribute(obj: &DataObject) -> i32 {
    obj.x
}
#[doc = "Access obj.y"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn get_y_attribute(obj: &DataObject) -> i32 {
    obj.y
}
#[doc = "Access obj.seconds"]
#[doc = " Depyler: verified panic-free"]
#[doc = " Depyler: proven to terminate"]
pub fn get_seconds_attribute(obj: &DataObject) -> i32 {
    (obj.num_seconds() % 86400) as i32
}
