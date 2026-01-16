use crate::api::TestliteralsSingleton;

pub struct DefaultTestLiteralsSingleton;

impl TestliteralsSingleton for DefaultTestLiteralsSingleton {
    fn getstringwithescapes(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_testliterals_singleton() -> Box<dyn TestliteralsSingleton> {
    Box::new(DefaultTestLiteralsSingleton)
}
