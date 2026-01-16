use crate::api::TestJsFieldsSingleton;

pub struct DefaultTestJsFieldsSingleton;

impl TestJsFieldsSingleton for DefaultTestJsFieldsSingleton {
    fn get_null_any(&mut self) -> mquickjs_rs::ValueRef<'_> {
        // v1 tests validate JS-visible behavior only; return `undefined`.
        mquickjs_rs::ValueRef::new(mquickjs_rs::mquickjs_ffi::JS_UNDEFINED)
    }
}

pub fn create_test_js_fields_singleton() -> Box<dyn TestJsFieldsSingleton> {
    Box::new(DefaultTestJsFieldsSingleton)
}
