//! Phase D (TODO 3a) 端到端实现层：struct 构造/透传 + C-like enum next/echo。

use crate::api::{Address, Color, Person, TestStructEnumSingleton};

pub struct DefaultTestStructEnumSingleton;

impl TestStructEnumSingleton for DefaultTestStructEnumSingleton {
    fn make_address(&mut self, street: String, num: i32) -> Address {
        Address { street, num }
    }

    fn echo_address(&mut self, a: Address) -> Address {
        a
    }

    fn make_person(&mut self, p: Person) -> Person {
        p
    }

    /// RED -> GREEN -> BLUE -> RED
    fn next_color(&mut self, c: Color) -> Color {
        match c {
            Color::Red => Color::Green,
            Color::Green => Color::Blue,
            Color::Blue => Color::Red,
        }
    }

    fn echo_color(&mut self, c: Color) -> Color {
        c
    }
}

pub fn create_test_struct_enum_singleton() -> Box<dyn TestStructEnumSingleton> {
    Box::new(DefaultTestStructEnumSingleton)
}
