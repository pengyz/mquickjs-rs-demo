use crate::api::TestFnSingleton;

pub struct DefaultTestFnSingleton;

impl TestFnSingleton for DefaultTestFnSingleton {
    fn add_int(&mut self, _a: i32, _b: i32) {
        // v1 tests validate JS-visible behavior only.
    }
}

pub fn create_test_fn_singleton() -> Box<dyn TestFnSingleton> {
    Box::new(DefaultTestFnSingleton)
}
