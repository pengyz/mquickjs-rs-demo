#[path = "../ridl_module_demo_default_impl.rs"]
mod ridl_module_demo_default_impl;

pub use ridl_module_demo_default_impl::*;

// Re-export generated singleton traits so glue can use stable paths: crate::impls::<Name>Singleton.
pub use crate::generated::impls::DemoSingleton;

struct DefaultDemoSingleton;

impl DemoSingleton for DefaultDemoSingleton {
    fn ping(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
        // Default demo singleton: no-op.
        // The JS smoke test only validates the singleton wiring path.
    }
}

pub fn create_demo_singleton() -> Box<dyn DemoSingleton> {
    Box::new(DefaultDemoSingleton)
}
