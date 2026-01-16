use crate::api::TestLiteralsSingleton;

pub struct DefaultTestLiteralsSingleton;

impl TestLiteralsSingleton for DefaultTestLiteralsSingleton {
    fn get_string_with_escapes(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_test_literals_singleton() -> *mut core::ffi::c_void {
    let b: Box<dyn TestLiteralsSingleton> = Box::new(DefaultTestLiteralsSingleton);
    let holder: Box<Box<dyn TestLiteralsSingleton>> = Box::new(b);
    Box::into_raw(holder) as *mut core::ffi::c_void
}
