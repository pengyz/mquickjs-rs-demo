use crate::api::TestLiteralsSingleton;

pub struct DefaultTestLiteralsSingleton;

impl TestLiteralsSingleton for DefaultTestLiteralsSingleton {
    fn get_string_with_escapes(&mut self) {
        // v1 tests validate JS-visible behavior only.
    }
}

pub fn create_test_literals_singleton() -> Box<dyn TestLiteralsSingleton> {
    Box::new(DefaultTestLiteralsSingleton)
}
