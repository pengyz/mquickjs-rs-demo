mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::{NodeClass, TestGcSingleton};

    pub fn node_constructor() -> Box<dyn NodeClass> {
        Box::new(crate::node_impl::DefaultNode::new())
    }

    pub fn create_test_gc_singleton() -> Box<dyn TestGcSingleton> {
        Box::new(crate::class_impl::DefaultTestGcSingleton)
    }
}

mod class_impl;
mod node_impl;
