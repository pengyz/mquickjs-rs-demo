mquickjs_rs::ridl_include_module!();

pub mod impls {
    use crate::api::FooClass;

    pub fn ping() -> i32 {
        1
    }

    pub fn foo_constructor() -> Box<dyn FooClass> {
        Box::new(crate::require_impl::DefaultFoo)
    }
}

mod require_impl;

// Test-only RIDL module for require() + module namespace integration.
//
// This crate is pulled into the app aggregate via root Cargo.toml dependencies.
