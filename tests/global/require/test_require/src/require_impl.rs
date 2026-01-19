use crate::api::FooClass;

pub struct DefaultFoo;

impl FooClass for DefaultFoo {
    fn value(&mut self) -> i32 {
        42
    }
}
