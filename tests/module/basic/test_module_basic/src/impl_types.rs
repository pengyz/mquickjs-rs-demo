use crate::api::MFooClass;

#[derive(Default)]
pub struct MFooImpl;

impl MFooClass for MFooImpl {
    fn add(&mut self, a: i32, b: i32) -> i32 {
        a + b
    }
}
