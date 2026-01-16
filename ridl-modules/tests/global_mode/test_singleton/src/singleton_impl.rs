use crate::api::TestsingletonSingleton;

pub struct DefaultTestSingletonSingleton;

impl TestsingletonSingleton for DefaultTestSingletonSingleton {
    fn ping(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_testsingleton_singleton() -> Box<dyn TestsingletonSingleton> {
    Box::new(DefaultTestSingletonSingleton)
}
