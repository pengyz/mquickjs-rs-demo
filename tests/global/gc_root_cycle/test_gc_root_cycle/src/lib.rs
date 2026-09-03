#![allow(clippy::needless_return)]

mod class_impl;
mod node_impl;

mquickjs_rs::ridl_include!("test_gc_root_cycle");

pub mod impls {
    use super::*;

    pub fn node_constructor() -> Box<dyn crate::api::NodeClass> {
        Box::new(node_impl::DefaultNode::new())
    }

    pub fn test_gc_singleton_constructor() -> Box<dyn crate::api::TestGcSingleton> {
        Box::new(class_impl::DefaultTestGcSingleton)
    }
}

pub fn init(ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext) {
    unsafe {
        // Safety: called once per context by the test harness.
        ridl_context_init(ctx);
    }

    unsafe {
        // Register module/singleton/class symbols.
        js_test_gc_root_cycle_module_class();
        js_test_gc_root_cycle_module_init(ctx);
    }
}
