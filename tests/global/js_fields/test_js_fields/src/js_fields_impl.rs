use crate::api::TestJsFieldsSingleton;

pub struct DefaultTestJsFieldsSingleton;

impl TestJsFieldsSingleton for DefaultTestJsFieldsSingleton {
    fn get_null_any(
        &mut self,
        _env: &mut mquickjs_rs::Env<'_>,
    ) -> () {
        unreachable!("any-return must use get_null_any_out")
    }

    fn get_null_any_out<'ctx>(
        &mut self,
        env: &mut mquickjs_rs::Env<'ctx>,
        out: &mut dyn for<'hs> FnMut(mquickjs_rs::handles::any::Any<'hs, 'ctx>),
    ) -> () {
        // v1 tests validate JS-visible behavior only; return `undefined`.
        let v = env.scope().value(mquickjs_rs::mquickjs_ffi::JS_UNDEFINED);
        out(mquickjs_rs::handles::any::Any::from_value(env.handle(v)))
    }
}

pub fn create_test_js_fields_singleton() -> Box<dyn TestJsFieldsSingleton> {
    Box::new(DefaultTestJsFieldsSingleton)
}