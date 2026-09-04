use crate::api::ConstantTestSingletonSingleton;

pub struct DefaultConstantTestSingleton {
    counter: i32,
    status: String,
}

impl DefaultConstantTestSingleton {
    pub fn new() -> Self {
        Self {
            counter: 0,
            status: "idle".to_string(),
        }
    }
}

impl ConstantTestSingletonSingleton for DefaultConstantTestSingleton {
    fn max_size(&self) -> i32 {
        100
    }

    fn pi(&self) -> f64 {
        3.14159
    }

    fn name(&self) -> String {
        "test".to_string()
    }

    fn enabled(&self) -> bool {
        true
    }

    fn counter(&self) -> i32 {
        self.counter
    }

    fn set_counter(&mut self, value: i32) {
        self.counter = value;
    }

    fn status(&self) -> String {
        self.status.clone()
    }

    fn set_status(&mut self, value: String) {
        self.status = value;
    }
}

pub fn create_constant_test_singleton_singleton() -> Box<dyn ConstantTestSingletonSingleton> {
    Box::new(DefaultConstantTestSingleton::new())
}