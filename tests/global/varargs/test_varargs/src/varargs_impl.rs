use crate::api::TestVarargsSingleton;

pub struct DefaultTestVarargsSingleton;

impl TestVarargsSingleton for DefaultTestVarargsSingleton {
    // String varargs arrive as owned Strings collected per element by the glue.
    fn join_all(&mut self, sep: String, parts: Vec<String>) -> String {
        parts.join(&sep)
    }
}

pub fn create_test_varargs_singleton() -> Box<dyn TestVarargsSingleton> {
    Box::new(DefaultTestVarargsSingleton)
}
