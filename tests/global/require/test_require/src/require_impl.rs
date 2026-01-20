use crate::api::FooClass;

use crate::api::BarClass;

pub struct DefaultFoo;

impl FooClass for DefaultFoo {
    fn value(&mut self) -> i32 {
        42
    }
}

pub struct DefaultBar;

impl BarClass for DefaultBar {
    fn value(&mut self) -> i32 {
        100
    }
}
