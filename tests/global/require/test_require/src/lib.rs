mquickjs_rs::ridl_include_module!();

pub mod impls {
    use crate::api::{BarClass, FooClass};

    pub fn ping() -> i32 {
        1
    }

    pub fn foo_constructor() -> Box<dyn FooClass> {
        Box::new(crate::require_impl::DefaultFoo)
    }

    pub fn bar_constructor() -> Box<dyn BarClass> {
        Box::new(crate::require_impl::DefaultBar)
    }
}

mod require_impl;

// Test-only RIDL module for require() + module namespace integration.
//
// This crate is pulled into the app aggregate via root Cargo.toml dependencies.
