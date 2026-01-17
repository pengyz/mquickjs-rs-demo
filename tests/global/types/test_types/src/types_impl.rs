use crate::api::TestTypesSingleton;

pub struct DefaultTestTypesSingleton;

impl TestTypesSingleton for DefaultTestTypesSingleton {
    fn echo_any(&mut self, _v: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>) {
        // v1 tests validate JS-visible behavior only.
    }
}

pub fn create_test_types_singleton() -> Box<dyn TestTypesSingleton> {
    Box::new(DefaultTestTypesSingleton)
}
