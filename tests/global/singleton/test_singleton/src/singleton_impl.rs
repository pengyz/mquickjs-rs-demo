use crate::api::TestPingSingleton;

pub struct DefaultTestPingSingleton;

impl TestPingSingleton for DefaultTestPingSingleton {
    fn ping(&mut self) -> String {
        "ok".to_string()
    }
}

pub fn create_test_ping_singleton() -> Box<dyn TestPingSingleton> {
    Box::new(DefaultTestPingSingleton)
}
