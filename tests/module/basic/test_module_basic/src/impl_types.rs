use crate::api::{MBarClass, MFooClass};

#[derive(Default)]
pub struct MFooImpl;

#[derive(Default)]
pub struct MBarImpl {
    pub v: i32,
}

impl MFooClass for MFooImpl {
    fn add(&mut self, a: i32, b: i32) -> i32 {
        a + b
    }

    fn make_bar(&mut self, v: i32) -> Box<dyn MBarClass> {
        Box::new(MBarImpl { v })
    }

    fn use_bar(&mut self, mut b: Box<dyn MBarClass>) -> i32 {
        b.get_v()
    }
}

impl MBarClass for MBarImpl {
    fn get_v(&mut self) -> i32 {
        self.v
    }
}
