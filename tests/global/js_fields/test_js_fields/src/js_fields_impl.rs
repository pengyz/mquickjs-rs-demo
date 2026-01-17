use crate::api::TestJsFieldsSingleton;

pub struct DefaultTestJsFieldsSingleton;

impl TestJsFieldsSingleton for DefaultTestJsFieldsSingleton {
    fn get_null_any(
        &mut self,
    ) -> mquickjs_rs::handles::global::Global<mquickjs_rs::handles::local::Value> {
        // v1 tests validate JS-visible behavior only; return `undefined`.
        let Some(h) = mquickjs_rs::context::ContextToken::current() else {
            panic!("getNullAny must run inside JS context");
        };
        let scope = h.enter_scope();
        mquickjs_rs::handles::global::Global::new(
            &scope,
            scope.value(mquickjs_rs::mquickjs_ffi::JS_UNDEFINED),
        )
    }
}

pub fn create_test_js_fields_singleton() -> Box<dyn TestJsFieldsSingleton> {
    Box::new(DefaultTestJsFieldsSingleton)
}
