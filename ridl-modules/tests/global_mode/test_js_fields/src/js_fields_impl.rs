use crate::api::TestJsFieldsSingleton;

pub struct DefaultTestJsFieldsSingleton;

impl TestJsFieldsSingleton for DefaultTestJsFieldsSingleton {
    fn get_null_any(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_test_js_fields_singleton() -> *mut core::ffi::c_void {
    let b: Box<dyn TestJsFieldsSingleton> = Box::new(DefaultTestJsFieldsSingleton);
    let holder: Box<Box<dyn TestJsFieldsSingleton>> = Box::new(b);
    Box::into_raw(holder) as *mut core::ffi::c_void
}
