use crate::api::TestSingletonSingleton;

pub struct DefaultTestSingletonSingleton;

impl TestSingletonSingleton for DefaultTestSingletonSingleton {
    fn ping(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_test_singleton_singleton() -> *mut core::ffi::c_void {
    let b: Box<dyn TestSingletonSingleton> = Box::new(DefaultTestSingletonSingleton);
    let holder: Box<Box<dyn TestSingletonSingleton>> = Box::new(b);
    Box::into_raw(holder) as *mut core::ffi::c_void
}
