use crate::api::TestJsFieldsSingleton;

pub struct DefaultTestJsFieldsSingleton;

impl TestJsFieldsSingleton for DefaultTestJsFieldsSingleton {
    fn get_null_any(
        &mut self,
        env: &mut mquickjs_rs::Env<'_>,
    ) -> mquickjs_rs::handles::return_safe::ReturnAny {
        // v1 tests validate JS-visible behavior only; return `undefined`.
        env.return_safe(env.scope().value(mquickjs_rs::mquickjs_ffi::JS_UNDEFINED))
    }
}

pub fn create_test_js_fields_singleton() -> Box<dyn TestJsFieldsSingleton> {
    Box::new(DefaultTestJsFieldsSingleton)
}
